/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
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
 */

#ifndef SCE_FORGE_ALGORITHM_H
#define SCE_FORGE_ALGORITHM_H

#include <stdbool.h>
#include <stdint.h>

/* Why a `may-fail` algorithm has no value to return. */
typedef enum {
    /* An integer result outside its declared width. */
    SCE_FORGE_ALGORITHM_OVERFLOW = 0,
    /* An integer `/` or `%` by zero. */
    SCE_FORGE_ALGORITHM_DIVIDE_BY_ZERO = 1,
    /* A buffer append past its declared capacity. */
    SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED = 2
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
