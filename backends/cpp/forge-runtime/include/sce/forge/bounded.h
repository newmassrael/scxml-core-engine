// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — bounded
// A list held together with the fact that it has no fewer elements than one
// bound and no more than another (docs/adr/0065 §6 R1, docs/adr/0077 §3). A
// codec's byte string holds at most its `sce:max-size`: the length field or the
// frame cannot say more, and the decoder refuses more, so a longer one has no
// frame. A value made of these types cannot present an element the encode
// would have to refuse, and the encode of a codec whose every refinement is a
// type refuses nothing (docs/adr/0081, step 4).
//
// `Bounded<L, Max, Min>` holds a list `L` and is checked once, by `make`; it
// has no mutable access, so its elements cannot leave the bounds once it is
// made. A `Bounded` whose bounds lie within another's converts to it unchecked,
// since its type already says so: to a view without a copy, and to an owning
// list by an explicit copy. `Bytes<Max, Min>` is the view a generated value
// presents a byte string through, and what its constructor takes; the value
// keeps its own copy.
//
// This mirrors the Rust runtime's `Bounded` (backends/rust/forge-runtime,
// `bounded.rs`), so the two languages' codecs hold the same refinement.

#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string_view>
#include <type_traits>
#include <utility>

#include "sce/forge/span.h"
#include "sce/forge/utf8.h"

namespace SCE::Forge {

/// The bound of a list, a byte string or a text the protocol states no bound
/// of (docs/adr/0077 §7): the largest `std::size_t`, past which no frame this
/// runtime reads has a length, so `Bounded` holds it with no second shape, as
/// the Rust runtime's `{ usize::MAX }` does.
inline constexpr std::size_t kUnbounded = SIZE_MAX;

template <typename L, std::size_t Max, std::size_t Min = 0> class Bounded;

namespace Detail {

/// Whether `T` is a `Bounded`.
template <typename T> struct IsBounded : std::false_type {};

template <typename L, std::size_t Max, std::size_t Min> struct IsBounded<Bounded<L, Max, Min>> : std::true_type {};

/// Whether `L` makes a list of `n` copies of a value: a list that owns its
/// elements, which can then hold the fewest a lower bound admits. The count is
/// asked as a value no constant gives: a constant zero such as `std::size_t{}`
/// is read by some compilers as a null pointer, which would let a view's
/// `(pointer, count)` constructor answer for a list that owns nothing.
template <typename L, typename = void> struct MakesCopies : std::false_type {};

template <typename L>
struct MakesCopies<L, std::void_t<decltype(L(std::declval<std::size_t>(), std::declval<typename L::value_type>()))>>
    : std::true_type {};

/// Whether `L` is made from the iterators of `M`: an owning list made as a
/// copy of what another list holds.
template <typename L, typename M, typename = void> struct CopiesFrom : std::false_type {};

template <typename L, typename M>
struct CopiesFrom<L, M, std::void_t<decltype(L(std::declval<const M &>().begin(), std::declval<const M &>().end()))>>
    : std::true_type {};

/// Whether `L` is a text: a list of `char`, which a `Bounded` holds only where
/// it is well-formed UTF-8, as a codec's `string` field is (docs/adr/0077 §3).
/// A byte string is a list of `std::uint8_t`, and holds any bytes.
template <typename L, typename = void> struct IsText : std::false_type {};

template <typename L>
struct IsText<L, std::void_t<typename L::value_type>> : std::is_same<std::remove_cv_t<typename L::value_type>, char> {};

/// Whether a list whose bounds are `Max2` and `Min2` lies within `Max` and `Min`.
template <std::size_t Max, std::size_t Min, std::size_t Max2, std::size_t Min2>
struct WithinBounds : std::bool_constant<Max2 <= Max && Min <= Min2> {};

/// What a value is read as where it is converted to `To` implicitly, as a
/// variable of type `To` initialized from it is. Never defined: it is read in
/// `noexcept` and `decltype` alone, to ask whether that conversion throws.
template <typename To> void converts_to(To) noexcept;

/// Whether the contiguous lists `a` and `b` hold equal elements in the same
/// order.
template <typename A, typename B> constexpr bool equal_elements(const A &a, const B &b) {
    const std::size_t count = static_cast<std::size_t>(a.size());
    if (count != static_cast<std::size_t>(b.size())) {
        return false;
    }
    const auto *left = a.data();
    const auto *right = b.data();
    for (std::size_t index = 0; index < count; ++index) {
        if (!(left[index] == right[index])) {
            return false;
        }
    }
    return true;
}

/// Whether `T` holds its elements contiguously and says so with `data()` and
/// `size()`, and is no `Bounded`: the other side of a comparison with one.
template <typename T, typename = void> struct IsContiguous : std::false_type {};

template <typename T>
struct IsContiguous<T,
                    std::void_t<decltype(std::declval<const T &>().data()), decltype(std::declval<const T &>().size())>>
    : std::negation<IsBounded<T>> {};

}  // namespace Detail

/// A list `L` of at least `Min` elements and at most `Max`.
template <typename L, std::size_t Max, std::size_t Min> class Bounded {
    static_assert(Min <= Max, "the bounds admit no list");

public:
    /// The list it holds.
    using list_type = L;

    /// The most elements it holds.
    static constexpr std::size_t most = Max;

    /// The fewest elements it holds.
    static constexpr std::size_t least = Min;

    /// The fewest elements the bounds admit: none, or `Min` value-made ones in
    /// a list that owns its elements. A view holds no list of its own, so it
    /// has no default where it must hold some.
    template <typename M = L, std::enable_if_t<Min == 0 || Detail::MakesCopies<M>::value, int> = 0>
    constexpr Bounded() noexcept(Min == 0 && std::is_nothrow_default_constructible<L>::value) : list_(fewest()) {}

    /// `list` held to the bounds, or nothing where it holds fewer than `Min`
    /// elements or more than `Max`, or is a text that is not well-formed
    /// UTF-8.
    [[nodiscard]] static constexpr std::optional<Bounded>
    make(L list) noexcept(std::is_nothrow_move_constructible<L>::value) {
        if (!admits(static_cast<std::size_t>(list.size()))) {
            return std::nullopt;
        }
        if constexpr (Detail::IsText<L>::value) {
            if (!is_well_formed_utf8(list.data(), static_cast<std::size_t>(list.size()))) {
                return std::nullopt;
            }
        }
        return Bounded(Checked{}, std::move(list));
    }

    /// The list a `Bounded` within these bounds holds, viewed or held as it
    /// is: the bounds are its type's, so nothing is checked.
    template <
        typename L2, std::size_t Max2, std::size_t Min2,
        std::enable_if_t<Detail::WithinBounds<Max, Min, Max2, Min2>::value && std::is_convertible<const L2 &, L>::value,
                         int> = 0>
    constexpr Bounded(const Bounded<L2, Max2, Min2> &other) noexcept(
        std::is_nothrow_constructible<L, const L2 &>::value)
        : list_(other.inner()) {}

    /// A copy of the elements a `Bounded` within these bounds holds, made in a
    /// list that owns them: explicit, since it copies.
    template <typename L2, std::size_t Max2, std::size_t Min2,
              std::enable_if_t<Detail::WithinBounds<Max, Min, Max2, Min2>::value &&
                                   !std::is_convertible<const L2 &, L>::value && Detail::CopiesFrom<L, L2>::value,
                               int> = 0>
    constexpr explicit Bounded(const Bounded<L2, Max2, Min2> &other)
        : list_(other.inner().begin(), other.inner().end()) {}

    /// The list.
    [[nodiscard]] constexpr const L &inner() const noexcept {
        return list_;
    }

    /// How many elements it holds, at least `Min` and at most `Max`.
    [[nodiscard]] constexpr std::size_t size() const noexcept {
        return static_cast<std::size_t>(list_.size());
    }

    /// Whether it holds no element.
    [[nodiscard]] constexpr bool empty() const noexcept {
        return size() == 0;
    }

    /// The first element, read only, of a list that holds its elements in a
    /// row; a list that does not has none, and is read by iteration.
    template <typename M = L>
    [[nodiscard]] constexpr auto data() const noexcept -> decltype(std::declval<const M &>().data()) {
        return list_.data();
    }

    /// The element at `index`, read only; `index` must be below `size()`. A
    /// list that cannot be indexed has none.
    template <typename M = L>
    [[nodiscard]] constexpr auto operator[](std::size_t index) const noexcept
        -> decltype(std::declval<const M &>()[index]) {
        return list_[index];
    }

    /// Where iteration starts: the first element.
    [[nodiscard]] constexpr auto begin() const noexcept -> decltype(std::declval<const L &>().begin()) {
        return list_.begin();
    }

    /// Where iteration ends: one past the last element.
    [[nodiscard]] constexpr auto end() const noexcept -> decltype(std::declval<const L &>().end()) {
        return list_.end();
    }

private:
    struct Checked {};

    constexpr Bounded(Checked, L list) noexcept(std::is_nothrow_move_constructible<L>::value)
        : list_(std::move(list)) {}

    /// Whether a list of `count` elements lies within the bounds.
    static constexpr bool admits(std::size_t count) noexcept {
        if constexpr (Min == 0) {
            return count <= Max;
        } else {
            return Min <= count && count <= Max;
        }
    }

    static constexpr L fewest() {
        if constexpr (Min == 0) {
            return L();
        } else {
            return L(Min, typename L::value_type{});
        }
    }

    L list_;
};

/// A byte string of at least `Min` bytes and at most `Max`, viewed: what a
/// generated value presents a codec's `bytes` element through, and what its
/// constructor takes (docs/adr/0077 §3, docs/adr/0081 step 4).
template <std::size_t Max, std::size_t Min = 0> using Bytes = Bounded<Span<const std::uint8_t>, Max, Min>;

/// A text of at least `Min` and at most `Max` UTF-8 bytes, viewed: what a
/// generated value presents a codec's `string` element through, and what its
/// constructor takes (docs/adr/0077 §3, docs/adr/0081 step 4). It holds
/// well-formed UTF-8 only, which `make` checks with the bound; the value keeps
/// its own copy, a `Bounded<std::string, Max, Min>`.
template <std::size_t Max, std::size_t Min = 0> using Text = Bounded<std::string_view, Max, Min>;

/// A list of at least `Min` and at most `Max` values of `T`, viewed: what a
/// generated value presents a codec's list of entries through, `T` the entry
/// codec's value (docs/adr/0065 §6 R2, docs/adr/0081 step 4). The value keeps
/// its own copy, a `Bounded<std::vector<T>, Max, Min>`.
template <typename T, std::size_t Max, std::size_t Min = 0> using List = Bounded<Span<const T>, Max, Min>;

/// Two lists of one count (docs/adr/0065 §6 R5): a codec that writes one number
/// for two lists holds both as this, so the number cannot disagree with either.
///
/// It is checked once, by `make`, and has no mutable access, so the two stay of
/// one count once it is made, as a `Bounded` stays within its bounds. Two lists
/// convert to two others as each list does, which keeps each count, so they
/// convert unchecked: to views without a copy, and to owning lists by an
/// explicit copy. This mirrors the Rust runtime's `Aligned`.
template <typename A, typename B> class Aligned {
public:
    /// Two lists made by default: the lists a count is shared by have no
    /// fewest entries, so both start empty.
    template <
        typename A2 = A, typename B2 = B,
        std::enable_if_t<std::is_default_constructible<A2>::value && std::is_default_constructible<B2>::value, int> = 0>
    constexpr Aligned() noexcept(std::is_nothrow_default_constructible<A>::value &&
                                 std::is_nothrow_default_constructible<B>::value)
        : first_(), second_() {}

    /// `first` and `second` as two lists of one count, or nothing where their
    /// counts differ.
    [[nodiscard]] static constexpr std::optional<Aligned>
    make(A first, B second) noexcept(std::is_nothrow_move_constructible<A>::value &&
                                     std::is_nothrow_move_constructible<B>::value) {
        if (static_cast<std::size_t>(first.size()) != static_cast<std::size_t>(second.size())) {
            return std::nullopt;
        }
        return Aligned(Checked{}, std::move(first), std::move(second));
    }

    /// The two lists two others of one count hold, viewed or held as they are.
    template <typename A2, typename B2,
              std::enable_if_t<std::is_convertible<const A2 &, A>::value && std::is_convertible<const B2 &, B>::value,
                               int> = 0>
    constexpr Aligned(const Aligned<A2, B2> &other) noexcept(std::is_nothrow_constructible<A, const A2 &>::value &&
                                                             std::is_nothrow_constructible<B, const B2 &>::value)
        : first_(other.first()), second_(other.second()) {}

    /// A copy of the two lists two others of one count hold, made in lists that
    /// own their entries: explicit, since it copies.
    template <
        typename A2, typename B2,
        std::enable_if_t<std::is_constructible<A, const A2 &>::value && std::is_constructible<B, const B2 &>::value &&
                             !(std::is_convertible<const A2 &, A>::value && std::is_convertible<const B2 &, B>::value),
                         int> = 0>
    constexpr explicit Aligned(const Aligned<A2, B2> &other) : first_(other.first()), second_(other.second()) {}

    /// The first list.
    [[nodiscard]] constexpr const A &first() const noexcept {
        return first_;
    }

    /// The second list.
    [[nodiscard]] constexpr const B &second() const noexcept {
        return second_;
    }

    /// How many entries each list holds.
    [[nodiscard]] constexpr std::size_t size() const noexcept {
        return static_cast<std::size_t>(first_.size());
    }

    /// Whether both lists are empty.
    [[nodiscard]] constexpr bool empty() const noexcept {
        return size() == 0;
    }

private:
    struct Checked {};

    constexpr Aligned(Checked, A first, B second) noexcept(std::is_nothrow_move_constructible<A>::value &&
                                                           std::is_nothrow_move_constructible<B>::value)
        : first_(std::move(first)), second_(std::move(second)) {}

    A first_;
    B second_;
};

/// Whether `a` and `b` hold equal first lists and equal second lists.
template <typename A, typename B, typename A2, typename B2>
constexpr bool operator==(const Aligned<A, B> &a, const Aligned<A2, B2> &b) {
    return a.first() == b.first() && a.second() == b.second();
}

/// Whether `a` and `b` differ.
template <typename A, typename B, typename A2, typename B2>
constexpr bool operator!=(const Aligned<A, B> &a, const Aligned<A2, B2> &b) {
    return !(a == b);
}

namespace Detail {

/// Whether `T` is a `Bounded` whose bounds lie within `Max` and `Min`: a list a
/// source may present for a list of those bounds, of any type it holds its
/// entries in.
template <typename T, std::size_t Max, std::size_t Min = 0> struct IsBoundedWithin : std::false_type {};

template <typename L, std::size_t Max2, std::size_t Min2, std::size_t Max, std::size_t Min>
struct IsBoundedWithin<Bounded<L, Max2, Min2>, Max, Min> : WithinBounds<Max, Min, Max2, Min2> {};

/// The type of the elements a list `T` holds, as iteration reads them.
template <typename T> using ElementOf = std::decay_t<decltype(*std::declval<const T &>().begin())>;

/// Whether a value of type `V` is a view of elements that belong to something
/// else: a `Bounded` over a list that owns nothing, a `Span`, a string view.
template <typename V> struct IsView : std::false_type {};

template <typename L, std::size_t Max, std::size_t Min>
struct IsView<Bounded<L, Max, Min>> : std::negation<MakesCopies<L>> {};

template <typename T> struct IsView<Span<T>> : std::true_type {};

template <typename C, typename T> struct IsView<std::basic_string_view<C, T>> : std::true_type {};

/// Whether what a read returning `R` presents lives on after the read: a
/// reference to an object, or a view returned by value. A list that owns its
/// elements, returned by value, dies at the end of the expression that read
/// it, and so does a view made of it; an encode that keeps a view of a read
/// past that expression asks this of the read.
template <typename R>
struct Lasting : std::disjunction<std::is_lvalue_reference<R>,
                                  std::conjunction<std::negation<std::is_reference<R>>, IsView<std::remove_cv_t<R>>>> {
};

/// What an encode keeps of a read returning `R` that it makes on some paths
/// only and uses past them, such as a body or a list one arm of a group holds:
/// the object a reference names, by its address, or what a read by value
/// returns, held whole. It keeps nothing until `keep` is called, and presents
/// what it keeps as a `const` object, as a `std::optional` does.
template <typename R> class Kept {
    using T = std::remove_cv_t<std::remove_reference_t<R>>;
    static constexpr bool by_address = std::is_lvalue_reference<R>::value;

public:
    /// Keeps `read`, what a read returning `R` returned.
    void keep(R read) noexcept(by_address || std::is_nothrow_move_constructible<T>::value) {
        if constexpr (by_address) {
            m_held = std::addressof(read);
        } else {
            m_held.emplace(std::move(read));
        }
    }

    /// Whether it keeps a read.
    [[nodiscard]] constexpr bool has_value() const noexcept {
        return static_cast<bool>(m_held);
    }

    /// Whether it keeps a read.
    constexpr explicit operator bool() const noexcept {
        return has_value();
    }

    /// What it keeps; it keeps a read.
    [[nodiscard]] constexpr const T &operator*() const noexcept {
        return *m_held;
    }

    /// What it keeps; it keeps a read.
    [[nodiscard]] constexpr const T *operator->() const noexcept {
        return std::addressof(**this);
    }

private:
    std::conditional_t<by_address, const T *, std::optional<T>> m_held{};
};

}  // namespace Detail

// A `Bounded` compares with what it holds, as a `std::vector` compares with
// another: the bounds are a fact about the value, not more of it, so
// `decoded.payload() == expected` reads as it would on the bare list.

/// Whether `a` and `b` hold equal elements in the same order.
template <typename L, std::size_t Max, std::size_t Min, typename L2, std::size_t Max2, std::size_t Min2>
constexpr bool operator==(const Bounded<L, Max, Min> &a, const Bounded<L2, Max2, Min2> &b) {
    return Detail::equal_elements(a, b);
}

/// Whether `a` and `b` differ.
template <typename L, std::size_t Max, std::size_t Min, typename L2, std::size_t Max2, std::size_t Min2>
constexpr bool operator!=(const Bounded<L, Max, Min> &a, const Bounded<L2, Max2, Min2> &b) {
    return !(a == b);
}

/// Whether `a` holds the elements of the contiguous list `b`, in order.
template <typename L, std::size_t Max, std::size_t Min, typename R,
          std::enable_if_t<Detail::IsContiguous<R>::value, int> = 0>
constexpr bool operator==(const Bounded<L, Max, Min> &a, const R &b) {
    return Detail::equal_elements(a, b);
}

/// Whether the contiguous list `a` holds the elements of `b`, in order.
template <typename R, typename L, std::size_t Max, std::size_t Min,
          std::enable_if_t<Detail::IsContiguous<R>::value, int> = 0>
constexpr bool operator==(const R &a, const Bounded<L, Max, Min> &b) {
    return Detail::equal_elements(a, b);
}

/// Whether `a` and the contiguous list `b` differ.
template <typename L, std::size_t Max, std::size_t Min, typename R,
          std::enable_if_t<Detail::IsContiguous<R>::value, int> = 0>
constexpr bool operator!=(const Bounded<L, Max, Min> &a, const R &b) {
    return !(a == b);
}

/// Whether the contiguous list `a` and `b` differ.
template <typename R, typename L, std::size_t Max, std::size_t Min,
          std::enable_if_t<Detail::IsContiguous<R>::value, int> = 0>
constexpr bool operator!=(const R &a, const Bounded<L, Max, Min> &b) {
    return !(a == b);
}

}  // namespace SCE::Forge
