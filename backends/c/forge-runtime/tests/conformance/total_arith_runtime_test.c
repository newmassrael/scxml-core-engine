/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/algorithm.h — the wrapping and saturating families held to their
 * definition (docs/adr/0063, section 1). For a type T of width W that holds
 * [m, M], and e the exact integer result:
 *
 *   wrapping    the r in [m, M] with r = e (mod 2^W);
 *   saturating  max(m, min(M, e)).
 *
 * Up to 32 bits every helper is run over a grid of values (all of them for
 * 8 bits) and compared with that definition, computed here by another route:
 * e is held exactly as a sign and a magnitude, which every result of two
 * 32-bit operands fits, and reduced by `%` on the magnitude. 64 bits have no
 * wider type to hold e in, so their cases are written out, each value worked
 * from the definition by hand; so are the cases the families exist for at
 * every width (the wrap point, saturation on each side, MIN * -1, neg(MIN),
 * MIN - 1, and a product past 2^W, which separates computing in the type
 * from computing in `int`). What a generated algorithm does with them is the
 * conformance fixtures' (docs/adr/0063, section 9), which this file does not
 * repeat.
 */

#include "sce/forge/algorithm.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

static int failures = 0;

static void expect_i(const char *what, int line, int64_t got, int64_t want) {
    if (got != want) {
        ++failures;
        fprintf(stderr, "FAIL %s:%d %s = %" PRId64 ", expected %" PRId64 "\n", __FILE__, line, what, got, want);
    }
}

static void expect_u(const char *what, int line, uint64_t got, uint64_t want) {
    if (got != want) {
        ++failures;
        fprintf(stderr, "FAIL %s:%d %s = %" PRIu64 ", expected %" PRIu64 "\n", __FILE__, line, what, got, want);
    }
}

#define EXPECT_I(call, want) expect_i(#call, __LINE__, (int64_t)(call), (int64_t)(want))
#define EXPECT_U(call, want) expect_u(#call, __LINE__, (uint64_t)(call), (uint64_t)(want))

/*
 * Every helper returns its own type, which is what a generated body stores
 * and compares: a helper answering `int` would pass every value check below
 * and still change what `==` on its result means.
 */
#define RETURNS_ITS_TYPE(SUFFIX, T)                                                                                    \
    _Static_assert(_Generic(sce_forge_wrapping_add_##SUFFIX(0, 0), T: 1, default: 0), "wrapping add " #SUFFIX);        \
    _Static_assert(_Generic(sce_forge_wrapping_sub_##SUFFIX(0, 0), T: 1, default: 0), "wrapping sub " #SUFFIX);        \
    _Static_assert(_Generic(sce_forge_wrapping_mul_##SUFFIX(0, 0), T: 1, default: 0), "wrapping mul " #SUFFIX);        \
    _Static_assert(_Generic(sce_forge_wrapping_neg_##SUFFIX(0), T: 1, default: 0), "wrapping neg " #SUFFIX);           \
    _Static_assert(_Generic(sce_forge_saturating_add_##SUFFIX(0, 0), T: 1, default: 0), "saturating add " #SUFFIX);    \
    _Static_assert(_Generic(sce_forge_saturating_sub_##SUFFIX(0, 0), T: 1, default: 0), "saturating sub " #SUFFIX);    \
    _Static_assert(_Generic(sce_forge_saturating_mul_##SUFFIX(0, 0), T: 1, default: 0), "saturating mul " #SUFFIX)

RETURNS_ITS_TYPE(u8, uint8_t);
RETURNS_ITS_TYPE(u16, uint16_t);
RETURNS_ITS_TYPE(u32, uint32_t);
RETURNS_ITS_TYPE(u64, uint64_t);
RETURNS_ITS_TYPE(i8, int8_t);
RETURNS_ITS_TYPE(i16, int16_t);
RETURNS_ITS_TYPE(i32, int32_t);
RETURNS_ITS_TYPE(i64, int64_t);
_Static_assert(_Generic(sce_forge_saturating_neg_i8(0), int8_t: 1, default: 0), "saturating neg i8");
_Static_assert(_Generic(sce_forge_saturating_neg_i16(0), int16_t: 1, default: 0), "saturating neg i16");
_Static_assert(_Generic(sce_forge_saturating_neg_i32(0), int32_t: 1, default: 0), "saturating neg i32");
_Static_assert(_Generic(sce_forge_saturating_neg_i64(0), int64_t: 1, default: 0), "saturating neg i64");

/* ------------------------------------------------------------------------- */
/* The definition, computed by another route                                 */
/* ------------------------------------------------------------------------- */

/* An exact integer as its sign and magnitude: every result of two operands of
 * at most 32 bits is below 2^64 in magnitude. A zero is never negative. */
typedef struct {
    bool negative;
    uint64_t magnitude;
} exact_t;

/* |v| for a v of magnitude below 2^63. */
static uint64_t magnitude_of(int64_t v) {
    return v < 0 ? (uint64_t)0 - (uint64_t)v : (uint64_t)v;
}

static exact_t exact(int64_t v) {
    exact_t e;
    e.negative = v < 0;
    e.magnitude = magnitude_of(v);
    return e;
}

static exact_t exact_product(int64_t a, int64_t b) {
    exact_t e;
    e.magnitude = magnitude_of(a) * magnitude_of(b);
    e.negative = e.magnitude != 0 && ((a < 0) != (b < 0));
    return e;
}

/* The r of the type whose least value is `lo` and width is `bits` (at most
 * 32) with r = e (mod 2^bits). */
static int64_t wrapped(exact_t e, int64_t lo, unsigned bits) {
    const uint64_t modulus = (uint64_t)1 << bits;
    uint64_t r = e.magnitude % modulus;
    if (e.negative && r != 0) {
        r = modulus - r;
    }
    if (lo < 0 && r >= modulus / 2) {
        return (int64_t)r - (int64_t)modulus;
    }
    return (int64_t)r;
}

/* max(lo, min(hi, e)). */
static int64_t saturated(exact_t e, int64_t lo, int64_t hi) {
    if (e.negative) {
        return e.magnitude > magnitude_of(lo) ? lo : -(int64_t)e.magnitude;
    }
    return e.magnitude > (uint64_t)hi ? hi : (int64_t)e.magnitude;
}

static void grid_case(const char *what, int64_t a, int64_t b, int64_t got, int64_t want) {
    if (got != want) {
        ++failures;
        fprintf(stderr, "FAIL %s(%" PRId64 ", %" PRId64 ") = %" PRId64 ", expected %" PRId64 "\n", what, a, b, got,
                want);
    }
}

/* Every operation of every pair of `values`, against the definition. */
#define AGAINST_THE_DEFINITION(SUFFIX, T, LO, HI, BITS)                                                                \
    static void SUFFIX##_against_the_definition(const T *values, size_t count) {                                       \
        size_t i;                                                                                                      \
        size_t j;                                                                                                      \
        for (i = 0; i < count; ++i) {                                                                                  \
            const T a = values[i];                                                                                     \
            const exact_t negated = exact(-(int64_t)a);                                                                \
            grid_case("wrapping neg " #SUFFIX, a, 0, sce_forge_wrapping_neg_##SUFFIX(a),                               \
                      wrapped(negated, (int64_t)(LO), BITS));                                                          \
            for (j = 0; j < count; ++j) {                                                                              \
                const T b = values[j];                                                                                 \
                const exact_t sum = exact((int64_t)a + (int64_t)b);                                                    \
                const exact_t difference = exact((int64_t)a - (int64_t)b);                                             \
                const exact_t product = exact_product(a, b);                                                           \
                grid_case("wrapping add " #SUFFIX, a, b, sce_forge_wrapping_add_##SUFFIX(a, b),                        \
                          wrapped(sum, (int64_t)(LO), BITS));                                                          \
                grid_case("wrapping sub " #SUFFIX, a, b, sce_forge_wrapping_sub_##SUFFIX(a, b),                        \
                          wrapped(difference, (int64_t)(LO), BITS));                                                   \
                grid_case("wrapping mul " #SUFFIX, a, b, sce_forge_wrapping_mul_##SUFFIX(a, b),                        \
                          wrapped(product, (int64_t)(LO), BITS));                                                      \
                grid_case("saturating add " #SUFFIX, a, b, sce_forge_saturating_add_##SUFFIX(a, b),                    \
                          saturated(sum, (int64_t)(LO), (int64_t)(HI)));                                               \
                grid_case("saturating sub " #SUFFIX, a, b, sce_forge_saturating_sub_##SUFFIX(a, b),                    \
                          saturated(difference, (int64_t)(LO), (int64_t)(HI)));                                        \
                grid_case("saturating mul " #SUFFIX, a, b, sce_forge_saturating_mul_##SUFFIX(a, b),                    \
                          saturated(product, (int64_t)(LO), (int64_t)(HI)));                                           \
            }                                                                                                          \
        }                                                                                                              \
    }

/* The saturating negation, which only a signed type has. */
#define SATURATING_NEG_AGAINST_THE_DEFINITION(SUFFIX, T, LO, HI)                                                       \
    static void SUFFIX##_saturating_neg_against_the_definition(const T *values, size_t count) {                        \
        size_t i;                                                                                                      \
        for (i = 0; i < count; ++i) {                                                                                  \
            grid_case("saturating neg " #SUFFIX, values[i], 0, sce_forge_saturating_neg_##SUFFIX(values[i]),           \
                      saturated(exact(-(int64_t)values[i]), (int64_t)(LO), (int64_t)(HI)));                            \
        }                                                                                                              \
    }

AGAINST_THE_DEFINITION(u8, uint8_t, 0, UINT8_MAX, 8)
AGAINST_THE_DEFINITION(u16, uint16_t, 0, UINT16_MAX, 16)
AGAINST_THE_DEFINITION(u32, uint32_t, 0, UINT32_MAX, 32)
AGAINST_THE_DEFINITION(i8, int8_t, INT8_MIN, INT8_MAX, 8)
AGAINST_THE_DEFINITION(i16, int16_t, INT16_MIN, INT16_MAX, 16)
AGAINST_THE_DEFINITION(i32, int32_t, INT32_MIN, INT32_MAX, 32)
SATURATING_NEG_AGAINST_THE_DEFINITION(i8, int8_t, INT8_MIN, INT8_MAX)
SATURATING_NEG_AGAINST_THE_DEFINITION(i16, int16_t, INT16_MIN, INT16_MAX)
SATURATING_NEG_AGAINST_THE_DEFINITION(i32, int32_t, INT32_MIN, INT32_MAX)

#define COUNT(xs) (sizeof(xs) / sizeof((xs)[0]))

static void every_8_bit_pair_meets_the_definition(void) {
    uint8_t unsigned_values[256];
    int8_t signed_values[256];
    int v;
    for (v = 0; v < 256; ++v) {
        unsigned_values[v] = (uint8_t)v;
        signed_values[v] = (int8_t)(v - 128);
    }
    u8_against_the_definition(unsigned_values, COUNT(unsigned_values));
    i8_against_the_definition(signed_values, COUNT(signed_values));
    i8_saturating_neg_against_the_definition(signed_values, COUNT(signed_values));
}

/* m, M, 0, 1, -1 and their neighbours, and the operands whose product is the
 * first past M (181 * 181 = 32761 and 182 * 182 = 33124 in 16 bits;
 * 46340 * 46340 = 2147395600 and 46341 * 46341 = 2147488281 in 32). */
static void the_bounds_of_16_and_32_bits_meet_the_definition(void) {
    static const uint16_t u16_values[] = {0, 1, 2, 181, 182, 255, 256, 257, 32767, 32768, 65534, 65535};
    static const int16_t i16_values[] = {INT16_MIN, INT16_MIN + 1, -256, -182, -181,  -2,       -1, 0, 1,
                                         2,         181,           182,  256,  32766, INT16_MAX};
    static const uint32_t u32_values[] = {0,     1,     2,           46340,       46341,          65535,
                                          65536, 65537, 0x7FFFFFFFu, 0x80000000u, UINT32_MAX - 1, UINT32_MAX};
    static const int32_t i32_values[] = {INT32_MIN, INT32_MIN + 1, -65536, -46341, -46340,        -2,       -1, 0, 1,
                                         2,         46340,         46341,  65536,  INT32_MAX - 1, INT32_MAX};
    u16_against_the_definition(u16_values, COUNT(u16_values));
    i16_against_the_definition(i16_values, COUNT(i16_values));
    i16_saturating_neg_against_the_definition(i16_values, COUNT(i16_values));
    u32_against_the_definition(u32_values, COUNT(u32_values));
    i32_against_the_definition(i32_values, COUNT(i32_values));
    i32_saturating_neg_against_the_definition(i32_values, COUNT(i32_values));
}

/* ------------------------------------------------------------------------- */
/* The cases the families exist for, worked by hand                         */
/* ------------------------------------------------------------------------- */

static void the_unsigned_widths_wrap_and_saturate_at_their_bounds(void) {
    /* 255 * 255 = 65025 = 254 * 256 + 1. */
    EXPECT_U(sce_forge_wrapping_add_u8(255, 1), 0);
    EXPECT_U(sce_forge_wrapping_sub_u8(0, 1), 255);
    EXPECT_U(sce_forge_wrapping_mul_u8(255, 255), 1);
    EXPECT_U(sce_forge_wrapping_mul_u8(16, 16), 0);
    EXPECT_U(sce_forge_wrapping_neg_u8(1), 255);
    EXPECT_U(sce_forge_wrapping_neg_u8(0), 0);
    EXPECT_U(sce_forge_saturating_add_u8(255, 1), 255);
    EXPECT_U(sce_forge_saturating_add_u8(254, 1), 255);
    EXPECT_U(sce_forge_saturating_add_u8(253, 1), 254);
    EXPECT_U(sce_forge_saturating_sub_u8(0, 1), 0);
    EXPECT_U(sce_forge_saturating_sub_u8(1, 1), 0);
    EXPECT_U(sce_forge_saturating_sub_u8(255, 254), 1);
    EXPECT_U(sce_forge_saturating_mul_u8(255, 255), 255);
    EXPECT_U(sce_forge_saturating_mul_u8(15, 17), 255);
    EXPECT_U(sce_forge_saturating_mul_u8(16, 16), 255);

    /* 65535 * 65535 = 2^32 - 2^17 + 1, past `int` where `int` is 32 bits:
     * the case that separates computing in the type from computing in `int`. */
    EXPECT_U(sce_forge_wrapping_add_u16(65535, 1), 0);
    EXPECT_U(sce_forge_wrapping_sub_u16(0, 1), 65535);
    EXPECT_U(sce_forge_wrapping_mul_u16(65535, 65535), 1);
    EXPECT_U(sce_forge_wrapping_mul_u16(256, 256), 0);
    EXPECT_U(sce_forge_wrapping_neg_u16(1), 65535);
    EXPECT_U(sce_forge_saturating_add_u16(65535, 1), 65535);
    EXPECT_U(sce_forge_saturating_add_u16(65534, 1), 65535);
    EXPECT_U(sce_forge_saturating_sub_u16(0, 65535), 0);
    EXPECT_U(sce_forge_saturating_mul_u16(65535, 65535), 65535);
    EXPECT_U(sce_forge_saturating_mul_u16(255, 257), 65535);
    EXPECT_U(sce_forge_saturating_mul_u16(256, 256), 65535);

    /* 0xFFFFFFFF * 0xFFFFFFFF = 2^64 - 2^33 + 1. */
    EXPECT_U(sce_forge_wrapping_add_u32(UINT32_MAX, 1), 0);
    EXPECT_U(sce_forge_wrapping_sub_u32(0, 1), UINT32_MAX);
    EXPECT_U(sce_forge_wrapping_mul_u32(0xFFFFFFFFu, 0xFFFFFFFFu), 1);
    EXPECT_U(sce_forge_wrapping_mul_u32(65536, 65536), 0);
    EXPECT_U(sce_forge_wrapping_neg_u32(1), UINT32_MAX);
    EXPECT_U(sce_forge_saturating_add_u32(UINT32_MAX, 1), UINT32_MAX);
    EXPECT_U(sce_forge_saturating_sub_u32(1, 2), 0);
    EXPECT_U(sce_forge_saturating_mul_u32(0xFFFFFFFFu, 0xFFFFFFFFu), UINT32_MAX);
    EXPECT_U(sce_forge_saturating_mul_u32(65535, 65537), UINT32_MAX);
    EXPECT_U(sce_forge_saturating_mul_u32(65536, 65536), UINT32_MAX);

    EXPECT_U(sce_forge_wrapping_add_u64(UINT64_MAX, 1), 0);
    EXPECT_U(sce_forge_wrapping_add_u64(UINT64_MAX, UINT64_MAX), UINT64_MAX - 1);
    EXPECT_U(sce_forge_wrapping_add_u64(UINT64_MAX - 1, 1), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_add_u64(0, 0), 0);
    EXPECT_U(sce_forge_wrapping_sub_u64(0, 1), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_sub_u64(0, UINT64_MAX), 1);
    EXPECT_U(sce_forge_wrapping_sub_u64(1, 2), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_sub_u64(UINT64_MAX, UINT64_MAX), 0);
    EXPECT_U(sce_forge_wrapping_sub_u64(UINT64_MAX, 0), UINT64_MAX);
    /* (2^64 - 1)^2 = 2^128 - 2^65 + 1; (2^32 + 1)(2^32 - 1) = 2^64 - 1. */
    EXPECT_U(sce_forge_wrapping_mul_u64(UINT64_MAX, UINT64_MAX), 1);
    EXPECT_U(sce_forge_wrapping_mul_u64(UINT64_MAX, 2), UINT64_MAX - 1);
    EXPECT_U(sce_forge_wrapping_mul_u64(UINT64_C(4294967296), UINT64_C(4294967296)), 0);
    EXPECT_U(sce_forge_wrapping_mul_u64(UINT64_C(4294967297), UINT64_C(4294967295)), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_mul_u64(1, UINT64_MAX), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_mul_u64(0, UINT64_MAX), 0);
    EXPECT_U(sce_forge_wrapping_neg_u64(0), 0);
    EXPECT_U(sce_forge_wrapping_neg_u64(1), UINT64_MAX);
    EXPECT_U(sce_forge_wrapping_neg_u64(UINT64_MAX), 1);
    EXPECT_U(sce_forge_wrapping_neg_u64(UINT64_C(9223372036854775808)), UINT64_C(9223372036854775808));
    EXPECT_U(sce_forge_saturating_add_u64(UINT64_MAX, 1), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_add_u64(UINT64_MAX, UINT64_MAX), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_add_u64(UINT64_MAX - 1, 1), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_add_u64(2, UINT64_MAX - 1), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_add_u64(UINT64_MAX - 1, 0), UINT64_MAX - 1);
    EXPECT_U(sce_forge_saturating_add_u64(0, 0), 0);
    EXPECT_U(sce_forge_saturating_sub_u64(0, 1), 0);
    EXPECT_U(sce_forge_saturating_sub_u64(1, UINT64_MAX), 0);
    EXPECT_U(sce_forge_saturating_sub_u64(1, 1), 0);
    EXPECT_U(sce_forge_saturating_sub_u64(UINT64_MAX, 1), UINT64_MAX - 1);
    EXPECT_U(sce_forge_saturating_sub_u64(UINT64_MAX, 0), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_MAX, UINT64_MAX), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_MAX, 2), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_MAX, 1), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_MAX, 0), 0);
    EXPECT_U(sce_forge_saturating_mul_u64(0, UINT64_MAX), 0);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_C(4294967296), UINT64_C(4294967296)), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_C(4294967296), UINT64_C(4294967297)), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_C(4294967297), UINT64_C(4294967295)), UINT64_MAX);
    EXPECT_U(sce_forge_saturating_mul_u64(UINT64_C(4294967296), UINT64_C(4294967295)), UINT64_C(18446744069414584320));
}

static void the_signed_widths_wrap_and_saturate_at_their_bounds(void) {
    /* 127 * 127 = 16129 = 63 * 256 + 1; -128 * -128 = 16384 = 64 * 256. */
    EXPECT_I(sce_forge_wrapping_add_i8(127, 1), -128);
    EXPECT_I(sce_forge_wrapping_sub_i8(-128, 1), 127);
    EXPECT_I(sce_forge_wrapping_mul_i8(-128, -1), -128);
    EXPECT_I(sce_forge_wrapping_mul_i8(127, 127), 1);
    EXPECT_I(sce_forge_wrapping_mul_i8(-128, -128), 0);
    EXPECT_I(sce_forge_wrapping_mul_i8(127, 2), -2);
    EXPECT_I(sce_forge_wrapping_neg_i8(-128), -128);
    EXPECT_I(sce_forge_wrapping_neg_i8(127), -127);
    EXPECT_I(sce_forge_saturating_add_i8(127, 1), 127);
    EXPECT_I(sce_forge_saturating_add_i8(-128, -1), -128);
    EXPECT_I(sce_forge_saturating_add_i8(-128, 127), -1);
    EXPECT_I(sce_forge_saturating_sub_i8(-128, 1), -128);
    EXPECT_I(sce_forge_saturating_sub_i8(0, -128), 127);
    EXPECT_I(sce_forge_saturating_sub_i8(-1, -128), 127);
    EXPECT_I(sce_forge_saturating_sub_i8(-2, 127), -128);
    EXPECT_I(sce_forge_saturating_mul_i8(-128, -1), 127);
    EXPECT_I(sce_forge_saturating_mul_i8(-128, 2), -128);
    EXPECT_I(sce_forge_saturating_mul_i8(127, -1), -127);
    EXPECT_I(sce_forge_saturating_mul_i8(-64, 2), -128);
    EXPECT_I(sce_forge_saturating_mul_i8(64, 2), 127);
    EXPECT_I(sce_forge_saturating_neg_i8(-128), 127);
    EXPECT_I(sce_forge_saturating_neg_i8(127), -127);
    EXPECT_I(sce_forge_saturating_neg_i8(-127), 127);

    /* (2^15 - 1)^2 = 2^30 - 2^16 + 1. */
    EXPECT_I(sce_forge_wrapping_add_i16(INT16_MAX, 1), INT16_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i16(INT16_MIN, 1), INT16_MAX);
    EXPECT_I(sce_forge_wrapping_mul_i16(INT16_MIN, -1), INT16_MIN);
    EXPECT_I(sce_forge_wrapping_mul_i16(INT16_MAX, INT16_MAX), 1);
    EXPECT_I(sce_forge_wrapping_mul_i16(INT16_MIN, INT16_MIN), 0);
    EXPECT_I(sce_forge_wrapping_neg_i16(INT16_MIN), INT16_MIN);
    EXPECT_I(sce_forge_saturating_add_i16(INT16_MAX, 1), INT16_MAX);
    EXPECT_I(sce_forge_saturating_sub_i16(INT16_MIN, 1), INT16_MIN);
    EXPECT_I(sce_forge_saturating_mul_i16(INT16_MIN, -1), INT16_MAX);
    EXPECT_I(sce_forge_saturating_mul_i16(181, 181), 32761);
    EXPECT_I(sce_forge_saturating_mul_i16(182, 182), INT16_MAX);
    EXPECT_I(sce_forge_saturating_mul_i16(-182, 182), INT16_MIN);
    EXPECT_I(sce_forge_saturating_neg_i16(INT16_MIN), INT16_MAX);

    /* 2^16 * 2^15 = 2^31, the pattern of INT32_MIN. */
    EXPECT_I(sce_forge_wrapping_add_i32(INT32_MAX, 1), INT32_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i32(INT32_MIN, 1), INT32_MAX);
    EXPECT_I(sce_forge_wrapping_mul_i32(INT32_MIN, -1), INT32_MIN);
    EXPECT_I(sce_forge_wrapping_mul_i32(INT32_MAX, INT32_MAX), 1);
    EXPECT_I(sce_forge_wrapping_mul_i32(65536, 65536), 0);
    EXPECT_I(sce_forge_wrapping_mul_i32(65536, 32768), INT32_MIN);
    EXPECT_I(sce_forge_wrapping_neg_i32(INT32_MIN), INT32_MIN);
    EXPECT_I(sce_forge_saturating_add_i32(INT32_MAX, INT32_MAX), INT32_MAX);
    EXPECT_I(sce_forge_saturating_sub_i32(INT32_MIN, 1), INT32_MIN);
    EXPECT_I(sce_forge_saturating_mul_i32(INT32_MIN, -1), INT32_MAX);
    EXPECT_I(sce_forge_saturating_mul_i32(46340, 46340), 2147395600);
    EXPECT_I(sce_forge_saturating_mul_i32(46341, 46341), INT32_MAX);
    EXPECT_I(sce_forge_saturating_mul_i32(-46341, 46341), INT32_MIN);
    EXPECT_I(sce_forge_saturating_neg_i32(INT32_MIN), INT32_MAX);

    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MAX, 1), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MIN, -1), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MAX, INT64_MAX), -2);
    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MIN, INT64_MIN), 0);
    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MIN, INT64_MAX), -1);
    EXPECT_I(sce_forge_wrapping_add_i64(INT64_MAX - 1, 1), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_add_i64(-1, 1), 0);
    EXPECT_I(sce_forge_wrapping_sub_i64(INT64_MIN, 1), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_sub_i64(INT64_MAX, -1), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i64(0, INT64_MIN), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i64(-1, INT64_MAX), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i64(-1, INT64_MIN), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_sub_i64(INT64_MIN + 1, 1), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_sub_i64(0, 1), -1);
    /* (2^32 + 1)(2^32 - 1) = 2^64 - 1, which is -1 modulo 2^64. */
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MIN, -1), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MAX, 2), -2);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MAX, INT64_MAX), 1);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MIN, INT64_MIN), 0);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MAX, -1), INT64_MIN + 1);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_MIN + 1, -1), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_mul_i64(-1, -1), 1);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_C(4294967296), INT64_C(4294967296)), 0);
    EXPECT_I(sce_forge_wrapping_mul_i64(INT64_C(4294967297), INT64_C(4294967295)), -1);
    EXPECT_I(sce_forge_wrapping_neg_i64(INT64_MIN), INT64_MIN);
    EXPECT_I(sce_forge_wrapping_neg_i64(INT64_MAX), INT64_MIN + 1);
    EXPECT_I(sce_forge_wrapping_neg_i64(INT64_MIN + 1), INT64_MAX);
    EXPECT_I(sce_forge_wrapping_neg_i64(0), 0);
    EXPECT_I(sce_forge_wrapping_neg_i64(1), -1);
    EXPECT_I(sce_forge_wrapping_neg_i64(-1), 1);

    EXPECT_I(sce_forge_saturating_add_i64(INT64_MAX, 1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MIN, -1), INT64_MIN);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MAX, INT64_MAX), INT64_MAX);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MIN, INT64_MIN), INT64_MIN);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MIN, INT64_MAX), -1);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MAX - 1, 1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MIN + 1, -1), INT64_MIN);
    EXPECT_I(sce_forge_saturating_add_i64(INT64_MAX, -1), INT64_MAX - 1);
    EXPECT_I(sce_forge_saturating_add_i64(0, 0), 0);
    /* -1 - MIN = 2^63 - 1 is M itself; -2 - MAX = -2^63 - 1 is one past m. */
    EXPECT_I(sce_forge_saturating_sub_i64(INT64_MIN, 1), INT64_MIN);
    EXPECT_I(sce_forge_saturating_sub_i64(INT64_MAX, -1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_sub_i64(0, INT64_MIN), INT64_MAX);
    EXPECT_I(sce_forge_saturating_sub_i64(-1, INT64_MIN), INT64_MAX);
    EXPECT_I(sce_forge_saturating_sub_i64(-2, INT64_MIN), INT64_MAX - 1);
    EXPECT_I(sce_forge_saturating_sub_i64(0, INT64_MAX), INT64_MIN + 1);
    EXPECT_I(sce_forge_saturating_sub_i64(-1, INT64_MAX), INT64_MIN);
    EXPECT_I(sce_forge_saturating_sub_i64(-2, INT64_MAX), INT64_MIN);
    EXPECT_I(sce_forge_saturating_sub_i64(INT64_MIN + 1, 1), INT64_MIN);
    EXPECT_I(sce_forge_saturating_sub_i64(1, 0), 1);
    /* 3037000499^2 = 9223372030926249001 is below M = 9223372036854775807;
     * 3037000500^2 = 9223372037000250000 is past it. 2^31 * 2^32 = 2^63. */
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MIN, -1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MAX, 2), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MAX, -2), INT64_MIN);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MIN, 2), INT64_MIN);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MIN, 1), INT64_MIN);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MAX, -1), INT64_MIN + 1);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MIN + 1, -1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(-1, -1), 1);
    EXPECT_I(sce_forge_saturating_mul_i64(0, INT64_MIN), 0);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_MIN, 0), 0);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_C(4294967296), INT64_C(4294967296)), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(-INT64_C(4294967296), INT64_C(4294967296)), INT64_MIN);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_C(2147483648), INT64_C(4294967296)), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(-INT64_C(2147483648), INT64_C(4294967296)), INT64_MIN);
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_C(3037000499), INT64_C(3037000499)), INT64_C(9223372030926249001));
    EXPECT_I(sce_forge_saturating_mul_i64(INT64_C(3037000500), INT64_C(3037000500)), INT64_MAX);
    EXPECT_I(sce_forge_saturating_mul_i64(-INT64_C(3037000500), -INT64_C(3037000500)), INT64_MAX);
    EXPECT_I(sce_forge_saturating_neg_i64(INT64_MIN), INT64_MAX);
    EXPECT_I(sce_forge_saturating_neg_i64(INT64_MAX), INT64_MIN + 1);
    EXPECT_I(sce_forge_saturating_neg_i64(INT64_MIN + 1), INT64_MAX);
    EXPECT_I(sce_forge_saturating_neg_i64(0), 0);
    EXPECT_I(sce_forge_saturating_neg_i64(1), -1);
    EXPECT_I(sce_forge_saturating_neg_i64(-1), 1);
}

/* The two functions of the request (docs/adr/0063, Context), over a 32-bit
 * tick counter that wrapped between `last` and `now`. */
static void a_tick_counter_that_wraps_is_read_across_the_wrap(void) {
    const uint32_t last = UINT32_MAX - 1;
    const uint32_t now = 5;
    /* now - last = 5 - (2^32 - 2) = 7 - 2^32, which is 7 modulo 2^32. */
    EXPECT_U(sce_forge_wrapping_sub_u32(now, last), 7);
    EXPECT_U(sce_forge_saturating_sub_u32(10, sce_forge_wrapping_sub_u32(now, last)), 3);
    EXPECT_U(sce_forge_saturating_sub_u32(5, sce_forge_wrapping_sub_u32(now, last)), 0);
    EXPECT_U(sce_forge_wrapping_sub_u32(UINT32_MAX, 0), UINT32_MAX);
    EXPECT_U(sce_forge_wrapping_sub_u32(0, UINT32_MAX), 1);
}

/* The CHECKED product of two uint32 values (SCE_FORGE.md §3.4.1), which the
 * total family sits beside: (2^32 - 1)^2 passes INT64_MAX, so a product made
 * in int64_t would overflow, undefined, inside the helper meant to report it.
 * Made in uint64_t, it is reported. */
static void a_checked_uint32_product_past_int64_is_reported(void) {
    sce_forge_algorithm_failure_t f = {false, SCE_FORGE_ALGORITHM_OVERFLOW};
    EXPECT_U(sce_forge_checked_mul_u32(&f, 65535, 65537), UINT32_MAX);
    EXPECT_U(f.failed, false);
    EXPECT_U(sce_forge_checked_mul_u32(&f, 65536, 65536), 0);
    EXPECT_U(f.failed, true);
    sce_forge_algorithm_failure_t g = {false, SCE_FORGE_ALGORITHM_OVERFLOW};
    EXPECT_U(sce_forge_checked_mul_u32(&g, UINT32_MAX, UINT32_MAX), 0);
    EXPECT_U(g.failed, true);
    EXPECT_U(g.error == SCE_FORGE_ALGORITHM_OVERFLOW, true);
}

int main(void) {
    every_8_bit_pair_meets_the_definition();
    the_bounds_of_16_and_32_bits_meet_the_definition();
    the_unsigned_widths_wrap_and_saturate_at_their_bounds();
    the_signed_widths_wrap_and_saturate_at_their_bounds();
    a_tick_counter_that_wraps_is_read_across_the_wrap();
    a_checked_uint32_product_past_int64_is_reported();
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("total arithmetic runtime: all cases pass\n");
    return 0;
}
