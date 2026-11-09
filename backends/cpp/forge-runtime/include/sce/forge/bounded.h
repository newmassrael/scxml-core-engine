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
// has no mutable access to an element, so its elements cannot leave the bounds
// once it is made. A `Bounded` whose bounds lie within another's converts to it
// unchecked, since its type already says so: to a view without a copy, and to
// an owning list by an explicit copy. `Bytes<Max, Min>` is the view a generated
// value presents a byte string through, and what its constructor takes; the
// value keeps its own copy.
//
// A `Bounded` over a list that owns its elements is also the place a generated
// `sce-static` machine holds a text, a byte string and a list in, its bound in
// its type (docs/adr/0086, section 7.4): `OwnedText<Max>`, `OwnedBytes<Max>`
// and `OwnedList<T, Max>`, the owned counterparts of `Text`, `Bytes` and
// `List`, in a variable, a record's field and a payload's alike. Such a place
// grows only by `push`, which refuses past `Max`, is emptied by `clear`, starts
// at a literal by `literal`, whose length the compiler judges against the
// bound, and is read wherever the list it holds is expected, read only.
//
// This mirrors the Rust runtime's `Bounded` (backends/rust/forge-runtime,
// `bounded.rs`), so the two languages' codecs hold the same refinement, and
// the owned forms mirror `sce-portable-bytes`' `SceString`, `SceBytes` and
// `BoundedList`, so the two languages' machines hold the same places.

#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

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

/// The list a `Bounded` holds, lent read only wherever that list is expected:
/// a host lent a machine's text as a `const std::string &`, a function that
/// takes a `const std::vector<T> &`, a `std::string` made of it. Only a list
/// that owns its elements is lent so; a view converts to another view by its
/// constructor. The conversion is a base's and not a member template of
/// `Bounded`, since a conversion function template that yields a reference
/// is not deduced where a `std::string` is copy-initialized from it by every
/// compiler: GCC refuses `std::string s = text;` through one.
template <typename Derived, typename L, bool Lends> class LendsItsList {};

template <typename Derived, typename L> class LendsItsList<Derived, L, true> {
public:
    constexpr operator const L &() const noexcept {
        return static_cast<const Derived &>(*this).inner();
    }
};

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
template <typename L, std::size_t Max, std::size_t Min>
class Bounded : public Detail::LendsItsList<Bounded<L, Max, Min>, L, Detail::MakesCopies<L>::value> {
    static_assert(Min <= Max, "the bounds admit no list");

public:
    /// The list it holds.
    using list_type = L;

    /// The type of an element, as the list it holds names it.
    using value_type = typename L::value_type;

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

    /// The text of the string literal `text`, held in a text that owns it: its
    /// length, the `K - 1` bytes before the zero that ends a literal, is judged
    /// against the bounds where the literal is written, so a literal past them
    /// does not compile, and one within them is built whole, with nothing to
    /// refuse (docs/adr/0086, section 7.4). A zero inside the literal is one of
    /// its bytes. What a generated machine starts a text place at; the
    /// generator writes only well-formed UTF-8.
    template <std::size_t K, typename M = L,
              std::enable_if_t<Detail::IsText<M>::value && Detail::MakesCopies<M>::value, int> = 0>
    [[nodiscard]] static Bounded literal(const char (&text)[K]) {
        static_assert(K >= 1, "a string literal ends with the zero that is no byte of its text");
        static_assert(K - 1 <= Max, "the literal is longer than the bound its place holds");
        static_assert(Min <= K - 1, "the literal is shorter than the fewest its place holds");
        return Bounded(Checked{}, L(text, K - 1));
    }

    /// The `K` elements of `items`, held in a list that owns them: `K` is judged
    /// against the bounds where the elements are written, as a text literal's
    /// length is, so `literal({0x01, 0x02})` past the bound does not compile.
    /// What a generated machine starts a byte-string place at; an empty one
    /// starts at the default, which holds none.
    template <std::size_t K, typename M = L,
              std::enable_if_t<!Detail::IsText<M>::value && Detail::MakesCopies<M>::value, int> = 0>
    [[nodiscard]] static Bounded literal(const value_type (&items)[K]) {
        static_assert(K <= Max, "the literal holds more elements than the bound its place holds");
        static_assert(Min <= K, "the literal holds fewer elements than the fewest its place holds");
        return Bounded(Checked{}, L(items, items + K));
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

    /// Append `value` where the list holds fewer than `Max` elements, and answer
    /// whether it did: a full list refuses it and is left as it was, so the list
    /// never holds more than its type says (docs/adr/0086, section 7.4). What a
    /// machine's `<sce:append>` writes through. A list that owns its elements
    /// has one; a text has none, since a byte appended could leave it no UTF-8,
    /// and is written whole instead.
    template <typename V, typename M = L,
              std::enable_if_t<!Detail::IsText<M>::value && Detail::MakesCopies<M>::value, int> = 0>
    [[nodiscard]] bool push(V &&value) {
        if (size() >= Max) {
            return false;
        }
        list_.push_back(std::forward<V>(value));
        return true;
    }

    /// Hold no element: what a machine's `<sce:clear>` does to a list, which
    /// then takes up to `Max` elements again. Only a list whose bounds admit
    /// none has it.
    template <typename M = L, std::enable_if_t<Min == 0 && Detail::MakesCopies<M>::value, int> = 0>
    void clear() noexcept {
        list_.clear();
    }

    /// The text it holds, viewed, wherever a `std::string_view` is expected: a
    /// text template's hole, a `std::string` made of it. Only a text that owns
    /// its bytes is viewed so. `V` is the target type the conversion is asked
    /// for.
    template <typename V, std::enable_if_t<std::is_same<V, std::string_view>::value && Detail::IsText<L>::value &&
                                               Detail::MakesCopies<L>::value,
                                           int> = 0>
    operator V() const noexcept {
        return V(list_.data(), list_.size());
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

/// A text of at least `Min` and at most `Max` UTF-8 bytes, held in a string of
/// its own: the one type a `string[Max]` is held in by a generated machine's
/// variable, a record's field, a payload's and a schema unit's alike
/// (docs/adr/0086, section 7.4), and what `Text` views. The owned counterpart
/// of `Text`, as Rust's `SceString<N>` is of a borrowed `str`.
template <std::size_t Max, std::size_t Min = 0> using OwnedText = Bounded<std::string, Max, Min>;

/// A byte string of at least `Min` and at most `Max` bytes, held in a vector of
/// its own: the type a `bytes[Max]` is held in by a generated machine's
/// variable, a record's field and a composed payload's (docs/adr/0086, section
/// 7.4), and what `Bytes` views. The owned counterpart of `Bytes`, as Rust's
/// `SceBytes<N>` is.
template <std::size_t Max, std::size_t Min = 0> using OwnedBytes = Bounded<std::vector<std::uint8_t>, Max, Min>;

/// A list of at least `Min` and at most `Max` values of `T`, held in a vector of
/// its own: the type a `list<T>[Max]` is held in by a generated machine's
/// variable, a record's field, a payload's and a schema unit's alike
/// (docs/adr/0086, section 7.4), and what `List` views. The owned counterpart
/// of `List`, as Rust's `BoundedList<T, N>` is.
template <typename T, std::size_t Max, std::size_t Min = 0> using OwnedList = Bounded<std::vector<T>, Max, Min>;

/// The list of `map` of each element of `from`, in order, each element taken
/// as the `T` `map` answers, held to the upper bound of `from` and to no lower
/// one: the copy a machine makes of a list into a place whose element is
/// copied by a conversion of its own (a text into a larger bound), which the
/// place then takes by widening, its own upper bound being at least `from`'s
/// (docs/adr/0086, sections 5.1 and 7.4). Every list place of a machine admits
/// none (`OwnedList<T, N>`), so no lower bound is carried; a place of a lower
/// bound does not take the copy and does not compile. A place whose upper
/// bound does not hold `from`'s does not compile either, so the copy refuses
/// nothing: no more elements are pushed than `from` holds, which is at most
/// `Max`.
template <typename T, typename L, std::size_t Max, std::size_t Min, typename F>
OwnedList<T, Max> widenedMap(const Bounded<L, Max, Min> &from, F &&map) {
    OwnedList<T, Max> to;
    for (const auto &element : from) {
        if (!to.push(map(element))) {
            break;
        }
    }
    return to;
}

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

/// The iterator of a range `R`, as `begin` gives it on a `const R`.
template <typename R> using IteratorOf = decltype(std::declval<const R &>().begin());

/// Whether `R` is a range a source may present for a list no bound refines (a
/// begin and end pair, docs/adr/0081 §2): its `begin` and `end` compared by
/// `!=`, the iterator advanced by `++` and read by `*`. A range is read more
/// than once, so it is walked again from `begin`.
template <typename R, typename = void> struct IsRange : std::false_type {};

template <typename R>
struct IsRange<R, std::void_t<decltype(std::declval<const R &>().begin() != std::declval<const R &>().end()),
                              decltype(++std::declval<IteratorOf<R> &>()), decltype(*std::declval<IteratorOf<R> &>())>>
    : std::true_type {};

/// Whether `R` is a range whose every element converts to `T`.
template <typename R, typename T, typename = void> struct IsRangeOf : std::false_type {};

template <typename R, typename T>
struct IsRangeOf<R, T, std::enable_if_t<IsRange<R>::value>>
    : std::is_convertible<decltype(*std::declval<IteratorOf<R> &>()), T> {};

/// Whether walking the range `R` throws nothing: its `begin`, `end`, the
/// comparison, the advance and the read of an element.
template <typename R, typename = void> struct IsNothrowRange : std::false_type {};

template <typename R>
struct IsNothrowRange<R, std::enable_if_t<IsRange<R>::value>>
    : std::bool_constant<noexcept(std::declval<const R &>().begin()) && noexcept(std::declval<const R &>().end()) &&
                         noexcept(std::declval<const R &>().begin() != std::declval<const R &>().end()) &&
                         noexcept(++std::declval<IteratorOf<R> &>()) && noexcept(*std::declval<IteratorOf<R> &>())> {};

/// Whether walking the range `R` of elements that convert to `T`, and taking
/// each as `T`, throws nothing.
template <typename R, typename T, typename = void> struct IsNothrowRangeOf : std::false_type {};

template <typename R, typename T>
struct IsNothrowRangeOf<R, T, std::enable_if_t<IsRangeOf<R, T>::value>>
    : std::bool_constant<IsNothrowRange<R>::value && noexcept(converts_to<T>(*std::declval<IteratorOf<R> &>()))> {};

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

// A text compares with a string literal, which holds no size of its own and is
// read up to its first zero, as a machine compares a variable with the literal
// a guard writes (`title == "beta"`).

/// Whether the text `a` holds the characters of `b`, in order.
template <typename L, std::size_t Max, std::size_t Min, std::enable_if_t<Detail::IsText<L>::value, int> = 0>
bool operator==(const Bounded<L, Max, Min> &a, const char *b) {
    return std::string_view(a.data(), a.size()) == std::string_view(b);
}

/// Whether `a` holds the characters of the text `b`, in order.
template <typename L, std::size_t Max, std::size_t Min, std::enable_if_t<Detail::IsText<L>::value, int> = 0>
bool operator==(const char *a, const Bounded<L, Max, Min> &b) {
    return b == a;
}

/// Whether the text `a` and `b` differ.
template <typename L, std::size_t Max, std::size_t Min, std::enable_if_t<Detail::IsText<L>::value, int> = 0>
bool operator!=(const Bounded<L, Max, Min> &a, const char *b) {
    return !(a == b);
}

/// Whether `a` and the text `b` differ.
template <typename L, std::size_t Max, std::size_t Min, std::enable_if_t<Detail::IsText<L>::value, int> = 0>
bool operator!=(const char *a, const Bounded<L, Max, Min> &b) {
    return !(b == a);
}

}  // namespace SCE::Forge
