// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#pragma once

#include <cstdint>
#include <limits>
#include <type_traits>

namespace SCE::Forge {

/// Why a `may-fail` algorithm has no value to return.
enum class AlgorithmError : std::uint8_t {
    /// An integer result outside its declared width.
    Overflow,
    /// An integer `/` or `%` by zero.
    DivideByZero,
    /// A buffer append past its declared capacity. This backend's buffers
    /// grow past their capacity (SCE_FORGE.md §4.12), so it never reports
    /// one; the case exists because the failure has one name everywhere.
    CapacityExceeded,
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
    if constexpr (Detail::kNarrow<T>) {
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

}  // namespace Checked
}  // namespace SCE::Forge
