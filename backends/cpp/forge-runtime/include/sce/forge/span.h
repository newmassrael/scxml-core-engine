// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — span
// The view a generated C++ function takes a list or a byte string through
// (docs/adr/0059, section 4): `SCE::Forge::Span<const T>`, a pointer to the
// caller's first element and a count, owning nothing. It is read by `size()`,
// `data()`, indexing and iteration, and it is made from anything that holds
// its elements contiguously and says so with `data()` and `size()`: a
// `std::vector`, a `std::array`, a `std::string_view`, another `Span` and, in
// a caller built at C++20, a `std::span`. A built-in array converts as well.
//
// It is one type in every language standard. It is not `std::span` under
// C++20, because a generated header is compiled by translation units of both
// standards in one program, and a header whose meaning depends on the standard
// breaks the One Definition Rule there (docs/adr/0059, section 3).

#pragma once

#include <cstddef>
#include <type_traits>
#include <utility>

namespace SCE::Forge {

template <typename T> class Span;

namespace Detail {

/// Whether a `U *` may be viewed as a `T *`: only by adding qualifiers, as
/// `std::span` allows. An array of `U` is convertible to an array of `T`
/// exactly then, so a `Derived *` is not taken for a `Base *`, whose elements
/// would be read at the wrong stride.
template <typename U, typename T> struct IsQualificationOf : std::is_convertible<U (*)[], T (*)[]> {};

/// Whether `S` is a `Span` of something, which converts by its own
/// constructor rather than as a container.
template <typename S> struct IsSpan : std::false_type {};

template <typename U> struct IsSpan<Span<U>> : std::true_type {};

/// The pointer type `C`'s `data()` answers, when it has one.
template <typename C> using DataPointer = decltype(std::declval<C>().data());

/// The type `C`'s `size()` answers, when it has one.
template <typename C> using SizeType = decltype(std::declval<C>().size());

/// Whether `C` (an expression type, with its value category) holds elements
/// a `Span<T>` can view: it is not itself a `Span`, its `data()` answers a
/// pointer to objects that convert to `T` by qualification alone, and its
/// `size()` answers a count.
template <typename C, typename T, typename = void> struct IsContiguousSourceOf : std::false_type {};

template <typename C, typename T>
struct IsContiguousSourceOf<C, T, std::void_t<DataPointer<C>, SizeType<C>>>
    : std::conjunction<std::negation<IsSpan<std::remove_cv_t<std::remove_reference_t<C>>>>,
                       std::is_pointer<DataPointer<C>>, std::is_object<std::remove_pointer_t<DataPointer<C>>>,
                       IsQualificationOf<std::remove_pointer_t<DataPointer<C>>, T>,
                       std::is_convertible<SizeType<C>, std::size_t>> {};

}  // namespace Detail

/// A non-owning view of `size()` contiguous elements of type `T` starting at
/// `data()`. Copying it copies the view, never the elements, and a `const`
/// view still reads through to elements it does not make `const`, as
/// `std::span` does: what is `const` is spelled in `T`.
///
/// The elements belong to whatever the view was made from, which must outlive
/// it. A generated function takes its view as a parameter and does not keep
/// it past the call.
template <typename T> class Span {
public:
    using element_type = T;
    using value_type = std::remove_cv_t<T>;
    using size_type = std::size_t;
    using pointer = T *;
    using reference = T &;
    using iterator = T *;

    /// The empty view.
    constexpr Span() noexcept = default;

    /// The `size` elements starting at `data`.
    constexpr Span(T *data, std::size_t size) noexcept : data_(data), size_(size) {}

    /// Every element of a built-in array.
    template <std::size_t N> constexpr Span(T (&array)[N]) noexcept : data_(array), size_(N) {}

    /// The elements another view views, read with more qualifiers: a
    /// `Span<T>` is a `Span<const T>`.
    template <typename U, typename = std::enable_if_t<Detail::IsQualificationOf<U, T>::value>>
    constexpr Span(const Span<U> &other) noexcept : data_(other.data()), size_(other.size()) {}

    /// Every element of a contiguous container the caller may write through:
    /// `data()` and `size()` of `container`.
    template <typename C, typename = std::enable_if_t<Detail::IsContiguousSourceOf<C &, T>::value>>
    constexpr Span(C &container) : data_(container.data()), size_(static_cast<std::size_t>(container.size())) {}

    /// Every element of a contiguous container read as `const`, a temporary
    /// included, which is what lets a `Span<const T>` parameter take a
    /// `std::vector` built in the call. A temporary's elements last until the
    /// end of the full expression the call is in.
    template <typename C, typename = std::enable_if_t<Detail::IsContiguousSourceOf<const C &, T>::value>>
    constexpr Span(const C &container) : data_(container.data()), size_(static_cast<std::size_t>(container.size())) {}

    /// The first element viewed; a null pointer for a view made empty.
    constexpr T *data() const noexcept {
        return data_;
    }

    /// How many elements are viewed.
    constexpr std::size_t size() const noexcept {
        return size_;
    }

    /// Whether no element is viewed.
    constexpr bool empty() const noexcept {
        return size_ == 0;
    }

    /// The element at `index`, which must be below `size()`.
    constexpr T &operator[](std::size_t index) const noexcept {
        return data_[index];
    }

    /// The first element; the view must not be empty.
    constexpr T &front() const noexcept {
        return data_[0];
    }

    /// The last element; the view must not be empty.
    constexpr T &back() const noexcept {
        return data_[size_ - 1];
    }

    /// Where iteration starts: the first element.
    constexpr T *begin() const noexcept {
        return data_;
    }

    /// Where iteration ends: one past the last element.
    constexpr T *end() const noexcept {
        return data_ + size_;
    }

private:
    T *data_ = nullptr;
    std::size_t size_ = 0;
};

}  // namespace SCE::Forge
