// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — law
// Lists whose entries say their inputs by a law (docs/adr/0065 §6 R8).
//
// An entry codec can say a flag-input itself (R4a) while the list that holds it
// hands every entry a value from elsewhere: a flag of the parent bound to every
// entry, or, in a chain, whether the entry before has an identifier. The two
// agree or the value has no frame. Both are a language over the list that a
// small automaton decides: a state, the value each entry must say in it, and the
// state after the entry. A bound flag is the automaton whose state is the flag
// and never changes (`Uniform`); the entry before is the one whose state starts
// clear and becomes whether the entry has the identifier (`AfterKey`). A list
// with both runs them together (`Laws`).
//
// `Lawful<L, A>` holds a list with the state its automaton starts in, which is
// the value's to give where the start is not fixed: the flag the parent writes
// is read from it, so an empty list still holds one. It is checked once, by
// `make`, as `Bounded` is, and it has no mutable access, so its entries cannot
// leave the law once it is made. A `Lawful` of one law converts to one of the
// same law over a list its own converts to, unchecked: to a view without a
// copy, and to an owning list by an explicit copy.
//
// This mirrors the Rust runtime's `Lawful` (backends/rust/forge-runtime,
// `law.rs`), so the two languages' codecs hold the same refinement.

#pragma once

#include <cstddef>
#include <optional>
#include <tuple>
#include <type_traits>
#include <utility>

namespace SCE::Forge {

/// What a value gives a law whose start is fixed: nothing.
struct FixedStart {};

/// Fixed starts are all one.
constexpr bool operator==(FixedStart, FixedStart) noexcept {
    return true;
}

/// Fixed starts never differ.
constexpr bool operator!=(FixedStart, FixedStart) noexcept {
    return false;
}

/// The law of a flag bound to every entry: each entry says the input `S` as the
/// start says it, and the start is the flag. `S::says(entry)` is the value the
/// entry says, or nothing where it says nothing of it (docs/adr/0065 §6 R8b).
template <class S> struct Uniform {
    /// The flag.
    using Start = bool;
    /// The flag, which no entry changes.
    using State = bool;

    /// The state before the first entry.
    static constexpr State start(Start start) noexcept {
        return start;
    }

    /// The state after `entry`, or nothing where `entry` says a value other
    /// than the one `state` asks of it.
    template <class E> static std::optional<State> step(State state, const E &entry) {
        const std::optional<bool> said = S::says(entry);
        if (said.has_value() && *said != state) {
            return std::nullopt;
        }
        return state;
    }

    /// The start a list that begins with `first` (none for an empty list)
    /// follows from: what the first entry says, else clear.
    template <class E> static Start start_of(const E *first) {
        if (first == nullptr) {
            return false;
        }
        return S::says(*first).value_or(false);
    }
};

/// The law of an input a chain supplies from the entry before
/// (`<sce:entry-flag-bind previous-entry-id>`): each entry says the input `S`
/// set exactly when the entry before is of the kind `K` (`K::matches(entry)`),
/// and the first says it clear. The start is fixed, so a value gives none.
template <class S, class K> struct AfterKey {
    /// Fixed.
    using Start = FixedStart;
    /// Whether the entry before is of the kind.
    using State = bool;

    /// The state before the first entry: no entry before it.
    static constexpr State start(Start) noexcept {
        return false;
    }

    /// The state after `entry`, or nothing where `entry` says a value other
    /// than the one `state` asks of it.
    template <class E> static std::optional<State> step(State state, const E &entry) {
        const std::optional<bool> said = S::says(entry);
        if (said.has_value() && *said != state) {
            return std::nullopt;
        }
        return static_cast<bool>(K::matches(entry));
    }

    /// The fixed start.
    template <class E> static Start start_of(const E *) {
        return {};
    }
};

/// Several laws over one list, run together: each entry follows each of them,
/// and the start is each one's, in order.
template <class... A> struct Laws {
    static_assert(sizeof...(A) > 0, "a list's law holds an automaton for each input");

    /// Each law's start.
    using Start = std::tuple<typename A::Start...>;
    /// Each law's state.
    using State = std::tuple<typename A::State...>;

    /// The state before the first entry.
    static constexpr State start(const Start &start) {
        return start_at(start, std::index_sequence_for<A...>{});
    }

    /// The state after `entry`, or nothing where it breaks any of the laws.
    template <class E> static std::optional<State> step(const State &state, const E &entry) {
        return step_at(state, entry, std::index_sequence_for<A...>{});
    }

    /// The start a list that begins with `first` follows from.
    template <class E> static Start start_of(const E *first) {
        return Start{A::start_of(first)...};
    }

private:
    template <std::size_t... I> static constexpr State start_at(const Start &start, std::index_sequence<I...>) {
        return State{A::start(std::get<I>(start))...};
    }

    template <class E, std::size_t... I>
    static std::optional<State> step_at(const State &state, const E &entry, std::index_sequence<I...>) {
        const std::tuple<std::optional<typename A::State>...> next{A::step(std::get<I>(state), entry)...};
        if (!(std::get<I>(next).has_value() && ...)) {
            return std::nullopt;
        }
        return State{*std::get<I>(next)...};
    }
};

namespace Detail {

/// Whether the entries of `list`, in order, follow the law `A` from `start`.
template <class A, class L> bool follows(const typename A::Start &start, const L &list) {
    typename A::State state = A::start(start);
    for (const auto &entry : list) {
        std::optional<typename A::State> next = A::step(state, entry);
        if (!next.has_value()) {
            return false;
        }
        state = *next;
    }
    return true;
}

}  // namespace Detail

/// A list `L` whose entries follow the law `A` from the start the value gives
/// (docs/adr/0065 §6 R8). The only way to hold one with a start of the
/// caller's is `make`, which runs the law over the entries; it reads as the
/// list it holds, and has no mutable access.
template <class L, class A> class Lawful {
public:
    /// The list it holds.
    using list_type = L;
    /// The law its entries follow.
    using law_type = A;
    /// What the value gives the law to start in.
    using start_type = typename A::Start;

    /// The list's own default, from the start its first entry says: the
    /// default entries of a list follow its law, which the generator holds
    /// the entry codec's default to. A view holds no list of its own, so it has
    /// no default where its bounds ask for entries.
    template <typename M = L, std::enable_if_t<std::is_default_constructible<M>::value, int> = 0>
    Lawful() : list_(), start_(A::start_of(first(list_))) {}

    /// `list` as a list whose entries follow `A` from `start`, or nothing where
    /// an entry says a value other than the one its place asks of it.
    [[nodiscard]] static std::optional<Lawful> make(start_type start, L list) {
        if (!Detail::follows<A>(start, list)) {
            return std::nullopt;
        }
        return Lawful(Checked{}, std::move(list), start);
    }

    /// The same entries and start over a list `L2` converts to: a view of an
    /// owning list, or a list within the bounds of another. The entries are the
    /// ones the law held, so nothing is checked.
    template <typename L2,
              std::enable_if_t<!std::is_same<L, L2>::value && std::is_convertible<const L2 &, L>::value, int> = 0>
    constexpr Lawful(const Lawful<L2, A> &other) noexcept(std::is_nothrow_constructible<L, const L2 &>::value)
        : list_(other.list()), start_(other.start()) {}

    /// A copy of the entries a `Lawful` of the same law holds, made in a list
    /// that owns them: explicit, since it copies.
    template <typename L2, std::enable_if_t<!std::is_same<L, L2>::value && !std::is_convertible<const L2 &, L>::value &&
                                                std::is_constructible<L, const L2 &>::value,
                                            int> = 0>
    constexpr explicit Lawful(const Lawful<L2, A> &other) : list_(other.list()), start_(other.start()) {}

    /// The start the entries follow from: for the law of a bound flag, the flag.
    [[nodiscard]] constexpr const start_type &start() const noexcept {
        return start_;
    }

    /// The list.
    [[nodiscard]] constexpr const L &list() const noexcept {
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

    Lawful(Checked, L list, start_type start) : list_(std::move(list)), start_(start) {}

    /// The first entry of `list`, or none.
    static auto first(const L &list) -> decltype(&*list.begin()) {
        if (list.begin() == list.end()) {
            return nullptr;
        }
        return &*list.begin();
    }

    L list_;
    start_type start_;
};

/// Whether `a` and `b` follow their law from the same start and hold equal
/// entries in the same order.
template <class L, class L2, class A> bool operator==(const Lawful<L, A> &a, const Lawful<L2, A> &b) {
    return a.start() == b.start() && a.list() == b.list();
}

/// Whether `a` and `b` differ.
template <class L, class L2, class A> bool operator!=(const Lawful<L, A> &a, const Lawful<L2, A> &b) {
    return !(a == b);
}

}  // namespace SCE::Forge
