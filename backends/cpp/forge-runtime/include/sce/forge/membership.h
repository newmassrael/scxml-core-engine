// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — membership
// Lists refined by the kinds of entry a chain's gates name (docs/adr/0065 §6
// R6). A field gated on `chain.has(k)` is on the wire exactly when an entry of
// the kind `k` is in the chain. The gates on one chain's entries, and the flags
// joined with them, decide its fields by their values together, so a value
// holds one arm for each assignment of them, and the arm says of each kind
// whether the chain has an entry of it. A value made of these types says which
// arm it is by the type it holds, so the fields the gates decide cannot
// disagree with the chain.
//
// `Chain<L, Holds, Lacks, Max, Min>` is the chain whole, with an entry of each
// kind of `Holds` and of none of `Lacks`: the arm's, for entries with content,
// which the value holds where they stand. `Placed<L, K, Max>` is the other
// entries and the place of the first entry of the kind `K` among them, where
// the one gate on a bare entry holds: the entry's identifier is all it is, so
// the codec derives it and the value holds its place.
//
// Each is checked once, by `make`, as a `Bounded` is, and has no mutable
// access, so it stays what it was checked to be. One converts to another of the
// same kinds whose bounds it lies within, unchecked, since its type already
// says so: to a view without a copy, and to an owning list by an explicit copy.
//
// A kind is a type `K` with `static bool matches(const E&)` for an entry `E`,
// generated for each identifier a gate looks for, over every source of the
// chain's entry codec. This mirrors the Rust runtime's `membership.rs`.

#pragma once

#include <cassert>
#include <cstddef>
#include <optional>
#include <type_traits>
#include <utility>

#include "sce/forge/bounded.h"

namespace SCE::Forge {

/// A set of kinds of entry, each a type `K` with `static bool matches(const
/// E&)`: what a `Chain` holds an entry of each of, or of none.
template <typename... K> struct Kinds {};

namespace Detail {

/// The place of the first entry of `list` of the kind `K`, or none.
template <typename K, typename L> constexpr std::optional<std::size_t> first_of_kind(const L &list) {
    std::size_t at = 0;
    for (const auto &entry : list) {
        if (K::matches(entry)) {
            return at;
        }
        ++at;
    }
    return std::nullopt;
}

/// What a set of kinds says of a list: whether it holds an entry of each, and
/// whether it holds an entry of none.
template <typename S> struct KindsOf;

template <typename... K> struct KindsOf<Kinds<K...>> {
    template <typename L> static constexpr bool each_in(const L &list) {
        return (first_of_kind<K>(list).has_value() && ...);
    }

    template <typename L> static constexpr bool none_in(const L &list) {
        return (!first_of_kind<K>(list).has_value() && ...);
    }
};

/// Whether `S` is the empty set of kinds.
template <typename S> struct IsNoKinds : std::false_type {};

template <> struct IsNoKinds<Kinds<>> : std::true_type {};

/// The place of the first entry of the kind `K` of a chain that may be
/// absent: an absent chain holds no entry of any kind.
template <typename K, typename L> constexpr std::optional<std::size_t> first_of_kind(const std::optional<L> &list) {
    if (!list.has_value()) {
        return std::nullopt;
    }
    return first_of_kind<K>(*list);
}

/// The chain `C` is, or holds where it is there: an arm whose kinds an absent
/// chain also has holds the chain as a `std::optional` of it.
template <typename C> struct ChainOf {
    using type = C;
};

template <typename C> struct ChainOf<std::optional<C>> {
    using type = C;
};

/// What an encode keeps of the list a chain holds, where a read returning `R`
/// presents the chain, or an optional of it: the list by reference where the
/// read names a chain that lives on, and a copy of it where the read returns
/// the chain by value, which dies at the end of the expression that read it.
/// Each arm of a group holds its chain as a type of its own kinds, and every
/// arm the same list.
template <typename R>
using InnerRead =
    std::conditional_t<std::is_lvalue_reference<R>::value, const typename ChainOf<std::decay_t<R>>::type::list_type &,
                       typename ChainOf<std::decay_t<R>>::type::list_type>;

}  // namespace Detail

/// A chain of at least `Min` entries and at most `Max`, held whole as the list
/// `L`, that holds an entry of each kind of `Holds` and an entry of none of
/// `Lacks` (docs/adr/0065 §6 R6b): the chain one arm of the gates on its entries
/// holds. The value holds each entry where it stands, so the first entry of a
/// kind is where the list has it, and nothing apart from the list says so.
template <typename L, typename Holds, typename Lacks, std::size_t Max, std::size_t Min = 0> class Chain {
    static_assert(Min <= Max, "the bounds admit no chain");

public:
    /// The list it holds.
    using list_type = L;

    /// The most entries it holds.
    static constexpr std::size_t most = Max;

    /// The fewest entries it holds.
    static constexpr std::size_t least = Min;

    /// The fewest entries the bounds admit, value-made, in a chain that holds
    /// no kind it must: the arm where no gate on the chain's entries holds. A
    /// value-made entry is of no kind such a chain lacks, which the generator
    /// that chooses the arm a value made by default holds keeps to, and which
    /// is asserted here.
    template <typename M = L,
              std::enable_if_t<Detail::IsNoKinds<Holds>::value && (Min == 0 || Detail::MakesCopies<M>::value), int> = 0>
    Chain() : list_(fewest()) {
        assert(Detail::KindsOf<Lacks>::none_in(list_) && "a value-made entry is of no kind the chain lacks");
    }

    /// `list` as a chain within the bounds with an entry of each kind of
    /// `Holds` and of none of `Lacks`, or nothing where it holds fewer than
    /// `Min` entries or more than `Max`, lacks a kind of `Holds`, or holds one
    /// of `Lacks`.
    [[nodiscard]] static constexpr std::optional<Chain>
    make(L list) noexcept(std::is_nothrow_move_constructible<L>::value) {
        const std::size_t count = static_cast<std::size_t>(list.size());
        if (count < Min || count > Max || !Detail::KindsOf<Holds>::each_in(list) ||
            !Detail::KindsOf<Lacks>::none_in(list)) {
            return std::nullopt;
        }
        return Chain(Checked{}, std::move(list));
    }

    /// The list a chain of the same kinds within these bounds holds, viewed or
    /// held as it is: the kinds and the bounds are its type's, so nothing is
    /// checked.
    template <
        typename L2, std::size_t Max2, std::size_t Min2,
        std::enable_if_t<Detail::WithinBounds<Max, Min, Max2, Min2>::value && std::is_convertible<const L2 &, L>::value,
                         int> = 0>
    constexpr Chain(const Chain<L2, Holds, Lacks, Max2, Min2> &other) noexcept(
        std::is_nothrow_constructible<L, const L2 &>::value)
        : list_(other.inner()) {}

    /// A copy of the entries a chain of the same kinds within these bounds
    /// holds, made in a list that owns them: explicit, since it copies.
    template <typename L2, std::size_t Max2, std::size_t Min2,
              std::enable_if_t<Detail::WithinBounds<Max, Min, Max2, Min2>::value &&
                                   !std::is_convertible<const L2 &, L>::value && Detail::CopiesFrom<L, L2>::value,
                               int> = 0>
    constexpr explicit Chain(const Chain<L2, Holds, Lacks, Max2, Min2> &other)
        : list_(other.inner().begin(), other.inner().end()) {}

    /// The list.
    [[nodiscard]] constexpr const L &inner() const noexcept {
        return list_;
    }

    /// How many entries it holds.
    [[nodiscard]] constexpr std::size_t size() const noexcept {
        return static_cast<std::size_t>(list_.size());
    }

    /// Whether it holds no entry.
    [[nodiscard]] constexpr bool empty() const noexcept {
        return size() == 0;
    }

    /// The entry at `index`, read only; `index` must be below `size()`.
    template <typename M = L>
    [[nodiscard]] constexpr auto operator[](std::size_t index) const noexcept
        -> decltype(std::declval<const M &>()[index]) {
        return list_[index];
    }

    /// Where iteration starts: the first entry.
    [[nodiscard]] constexpr auto begin() const noexcept -> decltype(std::declval<const L &>().begin()) {
        return list_.begin();
    }

    /// Where iteration ends: one past the last entry.
    [[nodiscard]] constexpr auto end() const noexcept -> decltype(std::declval<const L &>().end()) {
        return list_.end();
    }

private:
    struct Checked {};

    constexpr Chain(Checked, L list) noexcept(std::is_nothrow_move_constructible<L>::value) : list_(std::move(list)) {}

    static L fewest() {
        if constexpr (Min == 0) {
            return L();
        } else {
            return L(Min, typename L::value_type{});
        }
    }

    L list_;
};

/// Whether `a` and `b` hold equal entries in the same order.
template <typename L, typename H, typename A, std::size_t Max, std::size_t Min, typename L2, std::size_t Max2,
          std::size_t Min2>
constexpr bool operator==(const Chain<L, H, A, Max, Min> &a, const Chain<L2, H, A, Max2, Min2> &b) {
    return Detail::equal_elements(a.inner(), b.inner());
}

/// Whether `a` and `b` differ.
template <typename L, typename H, typename A, std::size_t Max, std::size_t Min, typename L2, std::size_t Max2,
          std::size_t Min2>
constexpr bool operator!=(const Chain<L, H, A, Max, Min> &a, const Chain<L2, H, A, Max2, Min2> &b) {
    return !(a == b);
}

/// A chain of at most `Max` entries that holds an entry of the kind `K`
/// (docs/adr/0065 §6 R6a): the others, held as the list `L`, at most `Max - 1`,
/// and the place of the first entry of the kind among them, no entry before the
/// place being of the kind. The entry of the kind is bare, its identifier all it
/// is, so the codec writes it from the kind and the value holds its place.
template <typename L, typename K, std::size_t Max> class Placed {
    static_assert(Max >= 1, "the chain cannot hold its named entry inside its bound");

public:
    /// The list of the others it holds.
    using list_type = L;

    /// The most entries the chain holds, the named one included.
    static constexpr std::size_t most = Max;

    /// The named entry alone.
    constexpr Placed() noexcept(std::is_nothrow_default_constructible<L>::value) : rest_(), place_(0) {}

    /// The entries `rest` with the named entry at `place` among them, or
    /// nothing where the chain they make holds more than `Max`, `place` is past
    /// the end of `rest`, or an entry before `place` is of the kind (the named
    /// entry is then that one, not the one at `place`).
    [[nodiscard]] static constexpr std::optional<Placed>
    make(L rest, std::size_t place) noexcept(std::is_nothrow_move_constructible<L>::value) {
        const std::size_t count = static_cast<std::size_t>(rest.size());
        if (count >= Max || place > count) {
            return std::nullopt;
        }
        const std::optional<std::size_t> first = Detail::first_of_kind<K>(rest);
        if (first.has_value() && *first < place) {
            return std::nullopt;
        }
        return Placed(Checked{}, std::move(rest), place);
    }

    /// The others and the place a chain of the same kind within this bound
    /// holds, viewed or held as they are.
    template <typename L2, std::size_t Max2,
              std::enable_if_t<(Max2 <= Max) && std::is_convertible<const L2 &, L>::value, int> = 0>
    constexpr Placed(const Placed<L2, K, Max2> &other) noexcept(std::is_nothrow_constructible<L, const L2 &>::value)
        : rest_(other.rest()), place_(other.place()) {}

    /// A copy of the others a chain of the same kind within this bound holds,
    /// made in a list that owns them, and the same place: explicit, since it
    /// copies.
    template <
        typename L2, std::size_t Max2,
        std::enable_if_t<
            (Max2 <= Max) && !std::is_convertible<const L2 &, L>::value && Detail::CopiesFrom<L, L2>::value, int> = 0>
    constexpr explicit Placed(const Placed<L2, K, Max2> &other)
        : rest_(other.rest().begin(), other.rest().end()), place_(other.place()) {}

    /// The entries other than the named one.
    [[nodiscard]] constexpr const L &rest() const noexcept {
        return rest_;
    }

    /// Where the named entry stands among the others: how many come before it.
    [[nodiscard]] constexpr std::size_t place() const noexcept {
        return place_;
    }

    /// How many entries the chain holds, the named one included.
    [[nodiscard]] constexpr std::size_t size() const noexcept {
        return static_cast<std::size_t>(rest_.size()) + 1;
    }

private:
    struct Checked {};

    constexpr Placed(Checked, L rest, std::size_t place) noexcept(std::is_nothrow_move_constructible<L>::value)
        : rest_(std::move(rest)), place_(place) {}

    L rest_;
    std::size_t place_;
};

/// Whether `a` and `b` hold equal others and the named entry at the same place.
template <typename L, typename K, std::size_t Max, typename L2, std::size_t Max2>
constexpr bool operator==(const Placed<L, K, Max> &a, const Placed<L2, K, Max2> &b) {
    return a.place() == b.place() && Detail::equal_elements(a.rest(), b.rest());
}

/// Whether `a` and `b` differ.
template <typename L, typename K, std::size_t Max, typename L2, std::size_t Max2>
constexpr bool operator!=(const Placed<L, K, Max> &a, const Placed<L2, K, Max2> &b) {
    return !(a == b);
}

}  // namespace SCE::Forge
