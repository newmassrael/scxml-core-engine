/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge — the integer arithmetic contract's runtime half.
 * See SCE_FORGE.md §3.4.1.
 *
 * An algorithm that declares `<sce:return may-fail="true">` returns its
 * `<sym>_result_t` with `ok` false and `why` naming the failure, and the
 * generator lowers each of its integer `+ - * / %` and unary `-` to one of
 * the `sce_forge_checked_<op>_<width>` helpers. A helper computes the
 * operation at the declared width, or records why it has no value there in
 * the algorithm's `sce_forge_algorithm_failure_t` and yields 0. The
 * generated body checks that record after every statement and returns the
 * failure. No helper executes an operation C leaves undefined: a division
 * by zero and a signed overflow are refused before they happen.
 *
 * An operation the author names as wrapping or saturating (`wrappingSub(a, b)`,
 * `saturatingSub(a, b)`; docs/adr/0063) has a value on every input, in a
 * `may-fail` body or not, and is lowered to one of the
 * `sce_forge_wrapping_<op>_<width>` and `sce_forge_saturating_<op>_<width>`
 * helpers, which take no failure record.
 */

#ifndef SCE_FORGE_ALGORITHM_H
#define SCE_FORGE_ALGORITHM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/forge/bytes.h" /* sce_forge_bytes_view_t, which a byte string's bound is held on */

/* Why a `may-fail` algorithm has no value to return. */
typedef enum {
    /* An integer result outside its declared width. */
    SCE_FORGE_ALGORITHM_OVERFLOW = 0,
    /* An integer `/` or `%` by zero. */
    SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO = 1,
    /* A buffer append past its declared capacity. */
    SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED = 2,
    /* A `<sce:require>` precondition that does not hold: an input outside
     * the algorithm's domain. */
    SCE_FORGE_ALGORITHM_PRECONDITION = 3,
    /* An index below 0 or not below its collection's length. */
    SCE_FORGE_ALGORITHM_OUT_OF_RANGE = 4
} sce_forge_algorithm_error_t;

/* The failure's name in the contract — the spelling every backend shares. */
static inline const char *sce_forge_algorithm_error_name(sce_forge_algorithm_error_t error) {
    switch (error) {
    case SCE_FORGE_ALGORITHM_OVERFLOW:
        return "overflow";
    case SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO:
        return "divide-by-zero";
    case SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED:
        return "capacity-exceeded";
    case SCE_FORGE_ALGORITHM_PRECONDITION:
        return "precondition";
    case SCE_FORGE_ALGORITHM_OUT_OF_RANGE:
        return "out-of-range";
    }
    return "overflow";
}

/*
 * The first failure a body's checked operations recorded. Later ones are
 * kept out: the statement that failed first is the one the caller learns
 * of, whatever else the same expression evaluated after it.
 */
typedef struct {
    bool failed;
    sce_forge_algorithm_error_t error;
} sce_forge_algorithm_failure_t;

static inline void sce_forge_algorithm_fail(sce_forge_algorithm_failure_t *f, sce_forge_algorithm_error_t error) {
    if (!f->failed) {
        f->failed = true;
        f->error = error;
    }
}

/*
 * `value`, handed back, when it holds no more than `capacity` UTF-8 bytes — the
 * bound a string variable of a `sce-static` machine declares — and "" with a
 * capacity failure recorded in `f` otherwise, which the statement around it
 * never uses. A C string is its bytes, so the count is `strlen`, which is what
 * every backend counts: a machine holds the same value wherever it runs.
 */
static inline const char *sce_forge_bounded_string(sce_forge_algorithm_failure_t *f, const char *value,
                                                   uint32_t capacity) {
    if (strlen(value) > (size_t)capacity) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED);
        return "";
    }
    return value;
}

/*
 * `value`, handed back, when it holds no more than `capacity` bytes — the bound a
 * byte string of a `sce-static` machine declares — and an empty view with a
 * capacity failure recorded in `f` otherwise, which the statement around it never
 * uses. The view carries its length, so the count is `len`, which is what every
 * backend counts.
 */
static inline sce_forge_bytes_view_t sce_forge_bounded_bytes(sce_forge_algorithm_failure_t *f,
                                                             sce_forge_bytes_view_t value, uint32_t capacity) {
    if (value.len > (size_t)capacity) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED);
        sce_forge_bytes_view_t none = {NULL, 0};
        return none;
    }
    return value;
}

/*
 * Every width up to 32 bits computes exactly in `int64_t`, where none of
 * these operations can overflow, and is then held to its own range.
 * Division truncates toward zero and `%` takes the dividend's sign, as C99
 * defines both. A remainder whose quotient overflows has no value either:
 * `MIN % -1` fails as `MIN / -1` does, on every backend.
 */
#define SCE_FORGE_CHECKED_NARROW(SUFFIX, T, LO, HI)                                                                    \
    static inline T sce_forge_checked_fit_##SUFFIX(sce_forge_algorithm_failure_t *f, int64_t v) {                      \
        if (v < (int64_t)(LO) || v > (int64_t)(HI)) {                                                                  \
            sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);                                                 \
            return 0;                                                                                                  \
        }                                                                                                              \
        return (T)v;                                                                                                   \
    }                                                                                                                  \
    static inline T sce_forge_checked_add_##SUFFIX(sce_forge_algorithm_failure_t *f, T a, T b) {                       \
        return sce_forge_checked_fit_##SUFFIX(f, (int64_t)a + (int64_t)b);                                             \
    }                                                                                                                  \
    static inline T sce_forge_checked_sub_##SUFFIX(sce_forge_algorithm_failure_t *f, T a, T b) {                       \
        return sce_forge_checked_fit_##SUFFIX(f, (int64_t)a - (int64_t)b);                                             \
    }                                                                                                                  \
    static inline T sce_forge_checked_mul_##SUFFIX(sce_forge_algorithm_failure_t *f, T a, T b) {                       \
        return sce_forge_checked_fit_##SUFFIX(f, (int64_t)a * (int64_t)b);                                             \
    }                                                                                                                  \
    static inline T sce_forge_checked_div_##SUFFIX(sce_forge_algorithm_failure_t *f, T a, T b) {                       \
        if (b == 0) {                                                                                                  \
            sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);                                           \
            return 0;                                                                                                  \
        }                                                                                                              \
        return sce_forge_checked_fit_##SUFFIX(f, (int64_t)a / (int64_t)b);                                             \
    }                                                                                                                  \
    static inline T sce_forge_checked_rem_##SUFFIX(sce_forge_algorithm_failure_t *f, T a, T b) {                       \
        if (b == 0) {                                                                                                  \
            sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);                                           \
            return 0;                                                                                                  \
        }                                                                                                              \
        (void)sce_forge_checked_fit_##SUFFIX(f, (int64_t)a / (int64_t)b);                                              \
        if (f->failed) {                                                                                               \
            return 0;                                                                                                  \
        }                                                                                                              \
        return (T)((int64_t)a % (int64_t)b);                                                                           \
    }                                                                                                                  \
    static inline T sce_forge_checked_neg_##SUFFIX(sce_forge_algorithm_failure_t *f, T a) {                            \
        return sce_forge_checked_fit_##SUFFIX(f, -(int64_t)a);                                                         \
    }

SCE_FORGE_CHECKED_NARROW(i8, int8_t, INT8_MIN, INT8_MAX)
SCE_FORGE_CHECKED_NARROW(i16, int16_t, INT16_MIN, INT16_MAX)
SCE_FORGE_CHECKED_NARROW(i32, int32_t, INT32_MIN, INT32_MAX)
SCE_FORGE_CHECKED_NARROW(u8, uint8_t, 0, UINT8_MAX)
SCE_FORGE_CHECKED_NARROW(u16, uint16_t, 0, UINT16_MAX)
SCE_FORGE_CHECKED_NARROW(u32, uint32_t, 0, UINT32_MAX)

#undef SCE_FORGE_CHECKED_NARROW

/*
 * 64 bits have no wider type to compute in, so each operation tests its
 * operands against the bounds before it runs (CERT INT32-C / INT30-C).
 */
static inline int64_t sce_forge_checked_add_i64(sce_forge_algorithm_failure_t *f, int64_t a, int64_t b) {
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a + b;
}

static inline int64_t sce_forge_checked_sub_i64(sce_forge_algorithm_failure_t *f, int64_t a, int64_t b) {
    if ((b < 0 && a > INT64_MAX + b) || (b > 0 && a < INT64_MIN + b)) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a - b;
}

static inline int64_t sce_forge_checked_mul_i64(sce_forge_algorithm_failure_t *f, int64_t a, int64_t b) {
    bool overflow;
    if (a == 0 || b == 0) {
        return 0;
    }
    if (a > 0) {
        overflow = b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a;
    } else {
        overflow = b > 0 ? a < INT64_MIN / b : b < INT64_MAX / a;
    }
    if (overflow) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a * b;
}

static inline int64_t sce_forge_checked_div_i64(sce_forge_algorithm_failure_t *f, int64_t a, int64_t b) {
    if (b == 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);
        return 0;
    }
    if (a == INT64_MIN && b == -1) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a / b;
}

static inline int64_t sce_forge_checked_rem_i64(sce_forge_algorithm_failure_t *f, int64_t a, int64_t b) {
    if (b == 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);
        return 0;
    }
    if (a == INT64_MIN && b == -1) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a % b;
}

static inline int64_t sce_forge_checked_neg_i64(sce_forge_algorithm_failure_t *f, int64_t a) {
    if (a == INT64_MIN) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return -a;
}

static inline uint64_t sce_forge_checked_add_u64(sce_forge_algorithm_failure_t *f, uint64_t a, uint64_t b) {
    if (a > UINT64_MAX - b) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a + b;
}

static inline uint64_t sce_forge_checked_sub_u64(sce_forge_algorithm_failure_t *f, uint64_t a, uint64_t b) {
    if (b > a) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a - b;
}

static inline uint64_t sce_forge_checked_mul_u64(sce_forge_algorithm_failure_t *f, uint64_t a, uint64_t b) {
    if (a != 0 && b > UINT64_MAX / a) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a * b;
}

static inline uint64_t sce_forge_checked_div_u64(sce_forge_algorithm_failure_t *f, uint64_t a, uint64_t b) {
    if (b == 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);
        return 0;
    }
    return a / b;
}

static inline uint64_t sce_forge_checked_rem_u64(sce_forge_algorithm_failure_t *f, uint64_t a, uint64_t b) {
    if (b == 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO);
        return 0;
    }
    return a % b;
}

static inline uint64_t sce_forge_checked_neg_u64(sce_forge_algorithm_failure_t *f, uint64_t a) {
    if (a != 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return a;
}

/*
 * The wrapping and saturating families (docs/adr/0063, section 1). For a type
 * T of width W that holds [m, M], and e the exact integer result of `add`,
 * `sub`, `mul` or `neg`:
 *
 *   wrapping    the r in [m, M] with r = e (mod 2^W): two's complement for a
 *               signed T;
 *   saturating  max(m, min(M, e)).
 *
 * Both have a value on every input, so neither takes a failure record, and
 * neither executes an operation C leaves undefined or implementation-defined
 * on any input. A saturating negation of an unsigned value would be 0 for
 * every input, and is not a helper: the generator refuses it where it is
 * written.
 */

/*
 * An unsigned wrapping operation is computed in WIDE, which is `unsigned int`
 * for 8 and 16 bits, `unsigned long` for 32 and `unsigned long long` for 64.
 * Each holds every value of its width (C11 5.2.4.2.1) and has a rank no lower
 * than `unsigned int`'s, so it is never promoted to `int`: a `uint16_t`
 * product such as 65535 * 65535 would overflow `int` (C11 6.3.1.1, 6.5p5).
 * WIDE's arithmetic is reduced modulo a power of two no smaller than 2^W, and
 * the conversion back to T reduces it modulo 2^W (C11 6.3.1.3p2), which is
 * the definition.
 */
#define SCE_FORGE_WRAPPING_UNSIGNED(SUFFIX, T, WIDE)                                                                   \
    static inline T sce_forge_wrapping_add_##SUFFIX(T a, T b) {                                                        \
        return (T)((WIDE)a + (WIDE)b);                                                                                 \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_sub_##SUFFIX(T a, T b) {                                                        \
        return (T)((WIDE)a - (WIDE)b);                                                                                 \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_mul_##SUFFIX(T a, T b) {                                                        \
        return (T)((WIDE)a * (WIDE)b);                                                                                 \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_neg_##SUFFIX(T a) {                                                             \
        return (T)((WIDE)0 - (WIDE)a);                                                                                 \
    }

SCE_FORGE_WRAPPING_UNSIGNED(u8, uint8_t, unsigned int)
SCE_FORGE_WRAPPING_UNSIGNED(u16, uint16_t, unsigned int)
SCE_FORGE_WRAPPING_UNSIGNED(u32, uint32_t, unsigned long)
SCE_FORGE_WRAPPING_UNSIGNED(u64, uint64_t, unsigned long long)

#undef SCE_FORGE_WRAPPING_UNSIGNED

/*
 * A signed type and the unsigned type of its width are one ring with two sets
 * of representatives, so a signed wrapping operation is the unsigned one on
 * its operands' two's-complement patterns. A conversion to an unsigned type is
 * defined modulo 2^W for every value (C11 6.3.1.3p2), which gives the pattern.
 * The way back is the branch in `from_bits`, because converting a pattern
 * above M to the signed type is implementation-defined (C11 6.3.1.3p3): a
 * pattern p above M stands for p - 2^W, which is -(UMAX - p) - 1, and every
 * step of that is in range.
 */
#define SCE_FORGE_WRAPPING_SIGNED(SUFFIX, T, USUFFIX, UT, HI, UHI)                                                     \
    static inline T sce_forge_wrapping_from_bits_##SUFFIX(UT bits) {                                                   \
        if (bits <= (UT)(HI)) {                                                                                        \
            return (T)bits;                                                                                            \
        }                                                                                                              \
        return (T)(-(T)((UT)(UHI) - bits) - 1);                                                                        \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_add_##SUFFIX(T a, T b) {                                                        \
        return sce_forge_wrapping_from_bits_##SUFFIX(sce_forge_wrapping_add_##USUFFIX((UT)a, (UT)b));                  \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_sub_##SUFFIX(T a, T b) {                                                        \
        return sce_forge_wrapping_from_bits_##SUFFIX(sce_forge_wrapping_sub_##USUFFIX((UT)a, (UT)b));                  \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_mul_##SUFFIX(T a, T b) {                                                        \
        return sce_forge_wrapping_from_bits_##SUFFIX(sce_forge_wrapping_mul_##USUFFIX((UT)a, (UT)b));                  \
    }                                                                                                                  \
    static inline T sce_forge_wrapping_neg_##SUFFIX(T a) {                                                             \
        return sce_forge_wrapping_from_bits_##SUFFIX(sce_forge_wrapping_neg_##USUFFIX((UT)a));                         \
    }

SCE_FORGE_WRAPPING_SIGNED(i8, int8_t, u8, uint8_t, INT8_MAX, UINT8_MAX)
SCE_FORGE_WRAPPING_SIGNED(i16, int16_t, u16, uint16_t, INT16_MAX, UINT16_MAX)
SCE_FORGE_WRAPPING_SIGNED(i32, int32_t, u32, uint32_t, INT32_MAX, UINT32_MAX)
SCE_FORGE_WRAPPING_SIGNED(i64, int64_t, u64, uint64_t, INT64_MAX, UINT64_MAX)

#undef SCE_FORGE_WRAPPING_SIGNED

/*
 * An unsigned saturating operation is the wrapping one where nothing wraps,
 * and M or 0 where it would. A sum wraps exactly when it comes out below an
 * operand, a difference when the subtrahend is the larger, and a product when
 * one operand exceeds M divided by the other (Hacker's Delight, section 2-13).
 * No test needs a wider type, so one shape serves every width, 64 bits
 * included.
 */
#define SCE_FORGE_SATURATING_UNSIGNED(SUFFIX, T, HI)                                                                   \
    static inline T sce_forge_saturating_add_##SUFFIX(T a, T b) {                                                      \
        T sum = sce_forge_wrapping_add_##SUFFIX(a, b);                                                                 \
        if (sum < a) {                                                                                                 \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        return sum;                                                                                                    \
    }                                                                                                                  \
    static inline T sce_forge_saturating_sub_##SUFFIX(T a, T b) {                                                      \
        if (b > a) {                                                                                                   \
            return (T)0;                                                                                               \
        }                                                                                                              \
        return sce_forge_wrapping_sub_##SUFFIX(a, b);                                                                  \
    }                                                                                                                  \
    static inline T sce_forge_saturating_mul_##SUFFIX(T a, T b) {                                                      \
        if (a != 0 && b > (T)(HI) / a) {                                                                               \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        return sce_forge_wrapping_mul_##SUFFIX(a, b);                                                                  \
    }

SCE_FORGE_SATURATING_UNSIGNED(u8, uint8_t, UINT8_MAX)
SCE_FORGE_SATURATING_UNSIGNED(u16, uint16_t, UINT16_MAX)
SCE_FORGE_SATURATING_UNSIGNED(u32, uint32_t, UINT32_MAX)
SCE_FORGE_SATURATING_UNSIGNED(u64, uint64_t, UINT64_MAX)

#undef SCE_FORGE_SATURATING_UNSIGNED

/*
 * A signed saturating operation tests its operands against the bounds before
 * it runs, as the 64-bit checked helpers do (CERT INT32-C), and answers the
 * bound the exact result passed instead of failing: no wider type is needed,
 * so one shape serves every width, 64 bits included. Each bound expression is
 * in range on the branch that evaluates it, a division by an operand is made
 * only where the operand is not 0, and no division is `MIN / -1`. A product
 * past a bound has the sign of the operands' product, which says which bound.
 */
#define SCE_FORGE_SATURATING_SIGNED(SUFFIX, T, LO, HI)                                                                 \
    static inline T sce_forge_saturating_add_##SUFFIX(T a, T b) {                                                      \
        if (b > 0 && a > (HI) - b) {                                                                                   \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        if (b < 0 && a < (LO) - b) {                                                                                   \
            return (T)(LO);                                                                                            \
        }                                                                                                              \
        return (T)(a + b);                                                                                             \
    }                                                                                                                  \
    static inline T sce_forge_saturating_sub_##SUFFIX(T a, T b) {                                                      \
        if (b < 0 && a > (HI) + b) {                                                                                   \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        if (b > 0 && a < (LO) + b) {                                                                                   \
            return (T)(LO);                                                                                            \
        }                                                                                                              \
        return (T)(a - b);                                                                                             \
    }                                                                                                                  \
    static inline T sce_forge_saturating_mul_##SUFFIX(T a, T b) {                                                      \
        bool past;                                                                                                     \
        if (a == 0 || b == 0) {                                                                                        \
            return (T)0;                                                                                               \
        }                                                                                                              \
        if (a > 0) {                                                                                                   \
            past = b > 0 ? a > (HI) / b : b < (LO) / a;                                                                \
        } else {                                                                                                       \
            past = b > 0 ? a < (LO) / b : b < (HI) / a;                                                                \
        }                                                                                                              \
        if (past && (a > 0) == (b > 0)) {                                                                              \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        if (past) {                                                                                                    \
            return (T)(LO);                                                                                            \
        }                                                                                                              \
        return (T)(a * b);                                                                                             \
    }                                                                                                                  \
    static inline T sce_forge_saturating_neg_##SUFFIX(T a) {                                                           \
        if (a == (LO)) {                                                                                               \
            return (T)(HI);                                                                                            \
        }                                                                                                              \
        return (T)(-a);                                                                                                \
    }

SCE_FORGE_SATURATING_SIGNED(i8, int8_t, INT8_MIN, INT8_MAX)
SCE_FORGE_SATURATING_SIGNED(i16, int16_t, INT16_MIN, INT16_MAX)
SCE_FORGE_SATURATING_SIGNED(i32, int32_t, INT32_MIN, INT32_MAX)
SCE_FORGE_SATURATING_SIGNED(i64, int64_t, INT64_MIN, INT64_MAX)

#undef SCE_FORGE_SATURATING_SIGNED

/*
 * A value stored where a narrower integer type is declared: the same value,
 * or 0 and an overflow when the type cannot hold it — never a wrapped one.
 * The value arrives as `int64_t` (from a signed type) or `uint64_t` (from an
 * unsigned one), either of which holds it exactly, so one helper per target
 * and signedness serves every source width. A pair whose every value fits
 * (`i64` from signed, `u64` from unsigned) has no helper: the generator
 * emits no check there.
 */
#define SCE_FORGE_CHECKED_INTO(SUFFIX, T, LO, HI)                                                                      \
    static inline T sce_forge_checked_narrow_##SUFFIX##_from_i(sce_forge_algorithm_failure_t *f, int64_t v) {          \
        if (v < (int64_t)(LO) || v > (int64_t)(HI)) {                                                                  \
            sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);                                                 \
            return 0;                                                                                                  \
        }                                                                                                              \
        return (T)v;                                                                                                   \
    }                                                                                                                  \
    static inline T sce_forge_checked_narrow_##SUFFIX##_from_u(sce_forge_algorithm_failure_t *f, uint64_t v) {         \
        if (v > (uint64_t)(HI)) {                                                                                      \
            sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);                                                 \
            return 0;                                                                                                  \
        }                                                                                                              \
        return (T)v;                                                                                                   \
    }

SCE_FORGE_CHECKED_INTO(i8, int8_t, INT8_MIN, INT8_MAX)
SCE_FORGE_CHECKED_INTO(i16, int16_t, INT16_MIN, INT16_MAX)
SCE_FORGE_CHECKED_INTO(i32, int32_t, INT32_MIN, INT32_MAX)
SCE_FORGE_CHECKED_INTO(u8, uint8_t, 0, UINT8_MAX)
SCE_FORGE_CHECKED_INTO(u16, uint16_t, 0, UINT16_MAX)
SCE_FORGE_CHECKED_INTO(u32, uint32_t, 0, UINT32_MAX)

#undef SCE_FORGE_CHECKED_INTO

/*
 * Whether index `i` is inside a collection of `len` elements; an out-of-range
 * failure recorded when it is not. The read is written after it
 * (`inside ? xs[i] : 0`), so an index outside is never read — an empty
 * collection included. The index arrives as `int64_t` (from a signed type)
 * or `uint64_t` (from an unsigned one), either of which holds it exactly.
 */
static inline bool sce_forge_checked_index_from_i(sce_forge_algorithm_failure_t *f, int64_t i, size_t len) {
    if (i < 0 || (uint64_t)i >= (uint64_t)len) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OUT_OF_RANGE);
        return false;
    }
    return true;
}

static inline bool sce_forge_checked_index_from_u(sce_forge_algorithm_failure_t *f, uint64_t i, size_t len) {
    if (i >= (uint64_t)len) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OUT_OF_RANGE);
        return false;
    }
    return true;
}

static inline int64_t sce_forge_checked_narrow_i64_from_u(sce_forge_algorithm_failure_t *f, uint64_t v) {
    if (v > (uint64_t)INT64_MAX) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return (int64_t)v;
}

static inline uint64_t sce_forge_checked_narrow_u64_from_i(sce_forge_algorithm_failure_t *f, int64_t v) {
    if (v < 0) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_OVERFLOW);
        return 0;
    }
    return (uint64_t)v;
}

#endif /* SCE_FORGE_ALGORITHM_H */
