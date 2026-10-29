// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — algorithm
// The integer arithmetic contract's runtime half. See SCE_FORGE.md §3.4.1.
//
// An algorithm that declares `<sce:return may-fail="true">` returns an
// `AlgorithmResult<T>`, and the generator lowers each of its integer
// `+ - * / %` and unary `-` to one of the `Checked` helpers. A helper
// computes the operation at the declared width, or records why it has no
// value there in the algorithm's `AlgorithmFailure` and yields 0. The
// generated body checks that record after every statement and returns the
// failure. No helper executes an operation the language leaves undefined:
// a division by zero and a signed overflow are refused before they happen,
// and no exception is thrown (the embedded profile has none).
//
// An operation the author names as wrapping or saturating (`wrappingSub(a, b)`,
// `saturatingSub(a, b)`; docs/adr/0063) has a value on every input, in a
// `may-fail` body or not, and is lowered to the `Wrapping` and `Saturating`
// helpers, which take no failure record.

#pragma once

#include <cstddef>
#include <cstdint>
#include <iterator>
#include <limits>
#include <optional>
#include <type_traits>

namespace SCE::Forge {

/// Why a `may-fail` algorithm has no value to return.
enum class AlgorithmError : std::uint8_t {
    /// An integer result outside its declared width.
    Overflow,
    /// An integer `/` or `%` by zero.
    DivideByZero,
    /// A buffer append past its declared capacity, or a string past the bound
    /// its variable declares (`Checked::bounded`). This backend's buffers grow
    /// past their capacity (SCE_FORGE.md §4.12), so a buffer never reports one;
    /// the failure has one name everywhere.
    CapacityExceeded,
    /// A `<sce:require>` precondition that does not hold: an input outside
    /// the algorithm's domain.
    Precondition,
    /// An index below 0 or not below its collection's length.
    OutOfRange,
};

/// The failure's name in the contract — the spelling every backend shares.
constexpr const char *contractName(AlgorithmError error) noexcept {
    switch (error) {
    case AlgorithmError::Overflow:
        return "overflow";
    case AlgorithmError::DivideByZero:
        return "divide-by-zero";
    case AlgorithmError::CapacityExceeded:
        return "capacity-exceeded";
    case AlgorithmError::Precondition:
        return "precondition";
    case AlgorithmError::OutOfRange:
        return "out-of-range";
    }
    return "overflow";
}

/// What a `may-fail` algorithm returns: its value, or why it has none.
template <typename T> class AlgorithmResult {
public:
    static AlgorithmResult success(T value) {
        return AlgorithmResult(value, true, AlgorithmError::Overflow);
    }

    static AlgorithmResult failure(AlgorithmError error) {
        return AlgorithmResult(T{}, false, error);
    }

    bool ok() const noexcept {
        return ok_;
    }

    /// The value; meaningful only when `ok()`.
    const T &value() const noexcept {
        return value_;
    }

    /// Why there is no value; meaningful only when `!ok()`.
    AlgorithmError error() const noexcept {
        return error_;
    }

private:
    AlgorithmResult(T value, bool ok, AlgorithmError error) : value_(value), ok_(ok), error_(error) {}

    T value_;
    bool ok_;
    AlgorithmError error_;
};

/// The first failure a body's checked operations recorded. Later ones are
/// kept out: the statement that failed first is the one the caller learns
/// of, whatever else the same expression evaluated after it.
class AlgorithmFailure {
public:
    void fail(AlgorithmError error) noexcept {
        if (!failed_) {
            failed_ = true;
            error_ = error;
        }
    }

    bool failed() const noexcept {
        return failed_;
    }

    AlgorithmError error() const noexcept {
        return error_;
    }

private:
    bool failed_ = false;
    AlgorithmError error_ = AlgorithmError::Overflow;
};

namespace Checked {
namespace Detail {

/// `v` as a `T` when it lies in `T`'s range; 0 and an overflow otherwise.
template <typename T> constexpr T fit(AlgorithmFailure &f, std::int64_t v) noexcept {
    if (v < static_cast<std::int64_t>(std::numeric_limits<T>::min()) ||
        v > static_cast<std::int64_t>(std::numeric_limits<T>::max())) {
        f.fail(AlgorithmError::Overflow);
        return T{};
    }
    return static_cast<T>(v);
}

// Every width up to 32 bits computes exactly in `int64_t`, where none of
// these operations can overflow, and is then held to its own range.
template <typename T> constexpr bool kNarrow = sizeof(T) <= 4;

template <typename T> constexpr bool kSigned = std::is_signed_v<T>;

}  // namespace Detail

/// The value of a call to another `may-fail` algorithm; or its failure
/// recorded in the calling body's `f`, and a `T{}` the statement around it
/// never uses — it returns the failure first (SCE_FORGE.md §3.4.1).
template <typename T> T take(AlgorithmFailure &f, const AlgorithmResult<T> &result) noexcept {
    if (!result.ok()) {
        f.fail(result.error());
        return T{};
    }
    return result.value();
}

/// The value a reading found, or an out-of-range failure when it found none:
/// the variant an enum's `from_underlying` found for a number, which is what
/// `<alias>.fromNumber(n)` answers (SCE_FORGE.md §4.12), and the value a lookup
/// whose miss is an error found for a key. A number outside the enum and a key
/// outside the table fail as an index outside a collection does.
template <typename T> AlgorithmResult<T> found(const std::optional<T> &value) {
    if (!value.has_value()) {
        return AlgorithmResult<T>::failure(AlgorithmError::OutOfRange);
    }
    return AlgorithmResult<T>::success(*value);
}

/// `value` when it holds no more than `capacity` bytes — the bound a string
/// variable of a `sce-static` machine declares, counted in UTF-8 bytes, and the
/// bound a byte string declares, counted in bytes — and an empty `S` with a
/// capacity failure recorded in `f` otherwise, which the statement around it
/// never uses. `S` is a string or a byte string whose `size()` counts bytes, as
/// `std::string`'s and `std::vector<std::uint8_t>`'s do, so a machine holds the
/// same value wherever it runs.
template <typename S> S bounded(AlgorithmFailure &f, S value, std::uint32_t capacity) {
    if (value.size() > capacity) {
        f.fail(AlgorithmError::CapacityExceeded);
        return S{};
    }
    return value;
}

template <typename T> constexpr T add(AlgorithmFailure &f, T a, T b) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if constexpr (Detail::kNarrow<T>) {
        return Detail::fit<T>(f, static_cast<std::int64_t>(a) + static_cast<std::int64_t>(b));
    } else if constexpr (Detail::kSigned<T>) {
        if ((b > 0 && a > std::numeric_limits<T>::max() - b) || (b < 0 && a < std::numeric_limits<T>::min() - b)) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a + b;
    } else {
        if (a > std::numeric_limits<T>::max() - b) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a + b;
    }
}

template <typename T> constexpr T sub(AlgorithmFailure &f, T a, T b) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if constexpr (Detail::kNarrow<T>) {
        return Detail::fit<T>(f, static_cast<std::int64_t>(a) - static_cast<std::int64_t>(b));
    } else if constexpr (Detail::kSigned<T>) {
        if ((b < 0 && a > std::numeric_limits<T>::max() + b) || (b > 0 && a < std::numeric_limits<T>::min() + b)) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a - b;
    } else {
        if (b > a) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a - b;
    }
}

template <typename T> constexpr T mul(AlgorithmFailure &f, T a, T b) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if constexpr (Detail::kNarrow<T> && !Detail::kSigned<T> && sizeof(T) == 4) {
        // (2^32 - 1)^2 passes INT64_MAX, so in `std::int64_t` this product
        // would be the overflow the helper reports, and undefined; it fits
        // `std::uint64_t`, where it is made instead.
        const std::uint64_t product = static_cast<std::uint64_t>(a) * static_cast<std::uint64_t>(b);
        if (product > std::numeric_limits<T>::max()) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return static_cast<T>(product);
    } else if constexpr (Detail::kNarrow<T>) {
        return Detail::fit<T>(f, static_cast<std::int64_t>(a) * static_cast<std::int64_t>(b));
    } else {
        if (a == 0 || b == 0) {
            return T{};
        }
        constexpr T kMax = std::numeric_limits<T>::max();
        constexpr T kMin = std::numeric_limits<T>::min();
        bool overflow = false;
        if constexpr (Detail::kSigned<T>) {
            // CERT INT32-C: each sign pairing divides the bound by an operand
            // whose quotient cannot itself overflow.
            if (a > 0) {
                overflow = b > 0 ? a > kMax / b : b < kMin / a;
            } else {
                overflow = b > 0 ? a < kMin / b : b < kMax / a;
            }
        } else {
            overflow = a > kMax / b;
        }
        if (overflow) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a * b;
    }
}

/// Truncated toward zero, as every backend divides (SCE_FORGE.md §3.4.1).
template <typename T> constexpr T div(AlgorithmFailure &f, T a, T b) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if (b == 0) {
        f.fail(AlgorithmError::DivideByZero);
        return T{};
    }
    if constexpr (Detail::kSigned<T>) {
        if (a == std::numeric_limits<T>::min() && b == -1) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
    }
    return static_cast<T>(a / b);
}

/// The sign of the dividend. A remainder whose quotient overflows has no
/// value either: `MIN % -1` fails as `MIN / -1` does, on every backend.
template <typename T> constexpr T rem(AlgorithmFailure &f, T a, T b) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if (b == 0) {
        f.fail(AlgorithmError::DivideByZero);
        return T{};
    }
    if constexpr (Detail::kSigned<T>) {
        if (a == std::numeric_limits<T>::min() && b == -1) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
    }
    return static_cast<T>(a % b);
}

template <typename T> constexpr T neg(AlgorithmFailure &f, T a) noexcept {
    static_assert(std::is_integral_v<T>, "checked arithmetic is integer arithmetic");
    if constexpr (Detail::kSigned<T>) {
        if (a == std::numeric_limits<T>::min()) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return static_cast<T>(-a);
    } else {
        if (a != 0) {
            f.fail(AlgorithmError::Overflow);
            return T{};
        }
        return a;
    }
}

/// The element of `c` at `i`, or a value-initialized element and an
/// out-of-range failure when `i` is below 0 or not below `c`'s length —
/// never a read past the end. `c` is anything `std::size` measures: a span,
/// a `std::array`, a vector, a built-in array.
template <typename C, typename I>
constexpr auto at(AlgorithmFailure &f, const C &c, I i) noexcept
    -> std::remove_cv_t<std::remove_reference_t<decltype(c[0])>> {
    static_assert(std::is_integral_v<I>, "an index is an integer");
    using T = std::remove_cv_t<std::remove_reference_t<decltype(c[0])>>;
    const auto n = static_cast<std::uint64_t>(std::size(c));
    bool inside = false;
    if constexpr (Detail::kSigned<I>) {
        inside = i >= 0 && static_cast<std::uint64_t>(i) < n;
    } else {
        inside = static_cast<std::uint64_t>(i) < n;
    }
    if (!inside) {
        f.fail(AlgorithmError::OutOfRange);
        return T{};
    }
    return c[static_cast<std::size_t>(i)];
}

/// A value of type `S` stored where a `T` is declared: the same value, or 0
/// and an overflow when `T` cannot hold it — never a wrapped one. Each
/// comparison is made in a 64-bit type of one signedness, which holds both
/// sides exactly.
template <typename T, typename S> constexpr T narrow(AlgorithmFailure &f, S v) noexcept {
    static_assert(std::is_integral_v<T> && std::is_integral_v<S>, "checked narrowing is integer narrowing");
    bool fits = false;
    if constexpr (Detail::kSigned<S> && Detail::kSigned<T>) {
        fits = static_cast<std::int64_t>(v) >= static_cast<std::int64_t>(std::numeric_limits<T>::min()) &&
               static_cast<std::int64_t>(v) <= static_cast<std::int64_t>(std::numeric_limits<T>::max());
    } else if constexpr (Detail::kSigned<S>) {
        fits = v >= 0 && static_cast<std::uint64_t>(v) <= static_cast<std::uint64_t>(std::numeric_limits<T>::max());
    } else {
        fits = static_cast<std::uint64_t>(v) <= static_cast<std::uint64_t>(std::numeric_limits<T>::max());
    }
    if (!fits) {
        f.fail(AlgorithmError::Overflow);
        return T{};
    }
    return static_cast<T>(v);
}

}  // namespace Checked

// The wrapping and saturating families (docs/adr/0063, section 1). For a type
// `T` of width W that holds [m, M], and e the exact integer result of `add`,
// `sub`, `mul` or `neg`:
//
//   wrapping    the r in [m, M] with r = e (mod 2^W): two's complement for a
//               signed `T`;
//   saturating  max(m, min(M, e)).
//
// Both have a value on every input, so neither takes a failure record, and
// neither executes an operation the language leaves undefined or
// implementation-defined on any input at C++17, the standard the interface
// compiles at (docs/adr/0059): `std::uint16_t` is promoted to `int`, where
// 65535 * 65535 overflows, and a conversion of a value a signed type cannot
// hold is implementation-defined before C++20.

namespace Detail {

/// Whether `T` is one of the eight integer types a document declares, which
/// are the types the two families are defined for.
template <typename T>
constexpr bool kFixedWidthInteger =
    std::is_same_v<T, std::int8_t> || std::is_same_v<T, std::int16_t> || std::is_same_v<T, std::int32_t> ||
    std::is_same_v<T, std::int64_t> || std::is_same_v<T, std::uint8_t> || std::is_same_v<T, std::uint16_t> ||
    std::is_same_v<T, std::uint32_t> || std::is_same_v<T, std::uint64_t>;

/// The unsigned type an operation of `T`'s width is computed in: `T`'s own
/// unsigned type, or `unsigned int` where that one would be promoted to `int`.
/// It holds every value of the width, is never promoted, and its arithmetic is
/// reduced modulo a power of two no smaller than 2^W.
template <typename T> using Ring = std::common_type_t<unsigned int, std::make_unsigned_t<T>>;

/// `a`'s two's-complement pattern, held in its ring. A conversion to an
/// unsigned type is defined modulo 2^W for every value, at every standard.
template <typename T> constexpr Ring<T> toRing(T a) noexcept {
    return static_cast<Ring<T>>(static_cast<std::make_unsigned_t<T>>(a));
}

/// The `T` whose two's-complement pattern is `r` reduced modulo 2^W. The
/// conversion to the unsigned type of the width is the reduction; for a
/// signed `T`, a pattern p above M stands for p - 2^W, which is
/// -(UMAX - p) - 1, and every step of that is in range.
template <typename T> constexpr T fromRing(Ring<T> r) noexcept {
    using U = std::make_unsigned_t<T>;
    const U bits = static_cast<U>(r);
    if constexpr (std::is_unsigned_v<T>) {
        return bits;
    } else {
        if (bits <= static_cast<U>(std::numeric_limits<T>::max())) {
            return static_cast<T>(bits);
        }
        return static_cast<T>(-static_cast<T>(static_cast<U>(std::numeric_limits<U>::max() - bits)) - 1);
    }
}

}  // namespace Detail

/// The wrapping family: an operation of `T` reduced modulo 2^W. A signed type
/// and the unsigned type of its width are one ring with two sets of
/// representatives, so each operation is made on the operands' patterns.
namespace Wrapping {

template <typename T> constexpr T add(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "wrapping arithmetic is defined for the fixed-width integer types");
    return Detail::fromRing<T>(Detail::toRing(a) + Detail::toRing(b));
}

template <typename T> constexpr T sub(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "wrapping arithmetic is defined for the fixed-width integer types");
    return Detail::fromRing<T>(Detail::toRing(a) - Detail::toRing(b));
}

template <typename T> constexpr T mul(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "wrapping arithmetic is defined for the fixed-width integer types");
    return Detail::fromRing<T>(Detail::toRing(a) * Detail::toRing(b));
}

template <typename T> constexpr T neg(T a) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "wrapping arithmetic is defined for the fixed-width integer types");
    return Detail::fromRing<T>(Detail::Ring<T>{0} - Detail::toRing(a));
}

}  // namespace Wrapping

/// The saturating family: an operation of `T` held to [m, M]. Each operation
/// tests its operands against the bounds before it runs (CERT INT32-C;
/// Hacker's Delight, section 2-13), so no width needs a wider type, and answers
/// the bound the exact result passed instead of failing. Each bound expression
/// is in range on the branch that evaluates it, a division by an operand is
/// made only where the operand is not 0, and no division is `MIN / -1`.
namespace Saturating {

template <typename T> constexpr T add(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "saturating arithmetic is defined for the fixed-width integer types");
    if constexpr (std::is_signed_v<T>) {
        constexpr T kMax = std::numeric_limits<T>::max();
        constexpr T kMin = std::numeric_limits<T>::min();
        if (b > 0 && a > kMax - b) {
            return kMax;
        }
        if (b < 0 && a < kMin - b) {
            return kMin;
        }
        return static_cast<T>(a + b);
    } else {
        // An unsigned sum wraps exactly when it comes out below an operand.
        const T sum = Wrapping::add<T>(a, b);
        if (sum < a) {
            return std::numeric_limits<T>::max();
        }
        return sum;
    }
}

template <typename T> constexpr T sub(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "saturating arithmetic is defined for the fixed-width integer types");
    if constexpr (std::is_signed_v<T>) {
        constexpr T kMax = std::numeric_limits<T>::max();
        constexpr T kMin = std::numeric_limits<T>::min();
        if (b < 0 && a > kMax + b) {
            return kMax;
        }
        if (b > 0 && a < kMin + b) {
            return kMin;
        }
        return static_cast<T>(a - b);
    } else {
        if (b > a) {
            return T{0};
        }
        return Wrapping::sub<T>(a, b);
    }
}

template <typename T> constexpr T mul(T a, T b) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "saturating arithmetic is defined for the fixed-width integer types");
    constexpr T kMax = std::numeric_limits<T>::max();
    if (a == 0 || b == 0) {
        return T{0};
    }
    if constexpr (std::is_signed_v<T>) {
        constexpr T kMin = std::numeric_limits<T>::min();
        bool past = false;
        if (a > 0) {
            past = b > 0 ? a > kMax / b : b < kMin / a;
        } else {
            past = b > 0 ? a < kMin / b : b < kMax / a;
        }
        // A product past a bound has the sign of the operands' product.
        if (past && (a > 0) == (b > 0)) {
            return kMax;
        }
        if (past) {
            return kMin;
        }
        return static_cast<T>(a * b);
    } else {
        if (b > kMax / a) {
            return kMax;
        }
        return Wrapping::mul<T>(a, b);
    }
}

/// A signed type's only: of an unsigned value it would be 0 for every input,
/// and the generator refuses it where it is written.
template <typename T> constexpr T neg(T a) noexcept {
    static_assert(Detail::kFixedWidthInteger<T>, "saturating arithmetic is defined for the fixed-width integer types");
    static_assert(std::is_signed_v<T>, "a saturating negation is defined for a signed type only");
    if (a == std::numeric_limits<T>::min()) {
        return std::numeric_limits<T>::max();
    }
    return static_cast<T>(-a);
}

}  // namespace Saturating
}  // namespace SCE::Forge
