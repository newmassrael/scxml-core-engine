/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge — the CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads
 * and writes (SCE_FORGE.md §4.6.1, docs/adr/0101): one definite-length map
 * whose keys are small unsigned integers and whose values are values of the
 * language, each in its one form: an integer, a float, a bool, a text or
 * byte string, an array, a map keyed by text, and a union's map of one
 * entry.
 *
 * Mirrors `backends/rust/forge-runtime/src/cbor.rs`, rule for rule. A
 * generated codec calls these; it spells no CBOR of its own.
 *
 * Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest
 * form and every float in the shortest precision that holds it exactly, so
 * two backends given the same value write the same bytes. Reading takes the
 * profile of definite lengths (docs/adr/0101 §3.1), and within it every
 * well-formed encoding of a value: a head in any valid length, a float in
 * any precision that holds the value. It refuses each input by the class
 * RFC 8949 §1.2 puts it in (docs/adr/0101 §3.3): an item that is not
 * well-formed or has an indefinite length is CBOR_MALFORMED, an item of a
 * major type, simple value or tag the place does not take is
 * CBOR_WRONG_TYPE, and a value outside its type is CBOR_OUT_OF_RANGE.
 *
 * No allocation and no recursion, as the rest of this runtime: a string is
 * read into the caller's fixed array, and the skip of an unknown value walks
 * a fixed stack of SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1 levels rather than
 * calling itself. A float's bits are moved with memcpy, and no function of
 * <math.h> is called, so no build links a math library for it.
 */

#ifndef SCE_FORGE_CBOR_H
#define SCE_FORGE_CBOR_H

#include <float.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/forge/codec.h"
#include "sce/text.h"

#ifdef __cplusplus
extern "C" {
#endif

enum {
    SCE_FORGE_CBOR_MAJOR_UNSIGNED = 0,
    /* The negative integer -1 - n for the argument n. */
    SCE_FORGE_CBOR_MAJOR_NEGATIVE = 1,
    SCE_FORGE_CBOR_MAJOR_BYTES = 2,
    SCE_FORGE_CBOR_MAJOR_TEXT = 3,
    SCE_FORGE_CBOR_MAJOR_ARRAY = 4,
    SCE_FORGE_CBOR_MAJOR_MAP = 5,
    SCE_FORGE_CBOR_MAJOR_SIMPLE = 7,
    /* The additional information of a half, a single and a double precision
     * float under major type 7 (RFC 8949 §3.3). */
    SCE_FORGE_CBOR_INFO_HALF = 25,
    SCE_FORGE_CBOR_INFO_SINGLE = 26,
    SCE_FORGE_CBOR_INFO_DOUBLE = 27,
    /* The deepest an unknown entry's value may nest before a skip refuses
     * it (SCE_FORGE.md §4.6.1). */
    SCE_FORGE_CBOR_MAX_SKIP_DEPTH = 16
};

/* ── Writing ───────────────────────────────────────────────────────── */

/* Write a head of `major` carrying `value`, in its shortest form. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_head(sce_forge_writer_t *w, uint8_t major, uint64_t value) {
    uint8_t head[9];
    size_t n = 0;
    size_t width = 0;
    const uint8_t m = (uint8_t)(major << 5);
    if (value < 24U) {
        head[n++] = (uint8_t)(m | (uint8_t)value);
    } else if (value <= 0xFFU) {
        head[n++] = (uint8_t)(m | 24U);
        width = 1;
    } else if (value <= 0xFFFFU) {
        head[n++] = (uint8_t)(m | 25U);
        width = 2;
    } else if (value <= 0xFFFFFFFFU) {
        head[n++] = (uint8_t)(m | 26U);
        width = 4;
    } else {
        head[n++] = (uint8_t)(m | 27U);
        width = 8;
    }
    while (width > 0U) {
        width--;
        head[n++] = (uint8_t)(value >> (8U * width));
    }
    return sce_forge_writer_write_bytes(w, head, n);
}

/* Write an unsigned integer. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_uint(sce_forge_writer_t *w, uint64_t value) {
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_UNSIGNED, value);
}

/* Write a signed integer: major type 0 for zero or more, major type 1 with
 * the argument -1 - n below zero, each with its shortest head (RFC 8949 §3.1
 * and §4.2.1). */
static inline sce_forge_codec_status_t sce_forge_cbor_write_int(sce_forge_writer_t *w, int64_t value) {
    if (value < 0) {
        /* -1 - value is at most INT64_MAX: it neither overflows nor loses a
         * bit as a uint64_t. */
        return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_NEGATIVE, (uint64_t)(-1 - value));
    }
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_UNSIGNED, (uint64_t)value);
}

/* The bits of a double, and the double of bits. */
static inline uint64_t sce_forge_cbor_double_bits(double x) {
    uint64_t bits;
    memcpy(&bits, &x, sizeof bits);
    return bits;
}

static inline double sce_forge_cbor_double_of_bits(uint64_t bits) {
    double x;
    memcpy(&x, &bits, sizeof x);
    return x;
}

static inline float sce_forge_cbor_single_of_bits(uint32_t bits) {
    float x;
    memcpy(&x, &bits, sizeof x);
    return x;
}

/* Whether `x` is a NaN: the one value not equal to itself. */
static inline bool sce_forge_cbor_is_nan(double x) {
    return x != x;
}

/* The half precision float that is `x` exactly, as its bits in `*out`, and
 * whether one is (IEEE 754 binary16: 5 bits of exponent, 10 of fraction,
 * subnormal below 2^-14). A NaN has none. */
static inline bool sce_forge_cbor_half_of(double x, uint16_t *out) {
    const uint64_t bits = sce_forge_cbor_double_bits(x);
    const uint16_t sign = (uint16_t)((bits >> 48) & 0x8000U);
    const int exponent = (int)((bits >> 52) & 0x7FFU);
    const uint64_t fraction = bits & ((UINT64_C(1) << 52) - 1U);
    int power;
    if (exponent == 0x7FF) {
        /* An infinity; a NaN is no half's. */
        *out = (uint16_t)(sign | 0x7C00U);
        return fraction == 0U;
    }
    if (exponent == 0) {
        /* Zero, or a double's subnormal, which is far below a half's least. */
        *out = sign;
        return fraction == 0U;
    }
    power = exponent - 1023;
    if (power >= -14 && power <= 15) {
        /* A half's normal number keeps the top 10 bits of the fraction and
         * nothing below them. */
        if ((fraction & ((UINT64_C(1) << 42) - 1U)) != 0U) {
            return false;
        }
        *out = (uint16_t)(sign | (uint16_t)((unsigned)(power + 15) << 10) | (uint16_t)(fraction >> 42));
        return true;
    }
    if (power >= -24 && power <= -15) {
        /* A half's subnormal number is k * 2^-24 for k from 1 to 1023. */
        const uint64_t significand = fraction | (UINT64_C(1) << 52);
        const unsigned shift = (unsigned)(28 - power);
        if ((significand & ((UINT64_C(1) << shift) - 1U)) != 0U) {
            return false;
        }
        *out = (uint16_t)(sign | (uint16_t)(significand >> shift));
        return true;
    }
    return false;
}

/* The single precision float that is `x` exactly, in `*out`, and whether
 * one is. A NaN has none. A finite double past a single's greatest is no
 * single, and is not converted: that conversion is undefined (C11 6.3.1.5). */
static inline bool sce_forge_cbor_single_of(double x, float *out) {
    const uint64_t bits = sce_forge_cbor_double_bits(x);
    if (sce_forge_cbor_is_nan(x)) {
        return false;
    }
    if ((bits & UINT64_C(0x7FFFFFFFFFFFFFFF)) == UINT64_C(0x7FF0000000000000)) {
        /* An infinity, which a single holds with its sign. */
        *out = sce_forge_cbor_single_of_bits((bits >> 63) != 0U ? 0xFF800000U : 0x7F800000U);
        return true;
    }
    if (x > (double)FLT_MAX || x < -(double)FLT_MAX) {
        return false;
    }
    *out = (float)x;
    return (double)*out == x;
}

/* Write a float in the shortest of half, single and double precision that
 * holds it exactly, and a NaN as f9 7e 00 (RFC 8949 §4.1 and §4.2.1,
 * docs/adr/0086 section 9.2). A float32 is written by its value as a double,
 * which a single always holds. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_float(sce_forge_writer_t *w, double x) {
    uint8_t item[9];
    size_t n = 0;
    size_t width = 0;
    uint64_t argument = 0;
    uint16_t half = 0;
    float single = 0.0F;
    if (sce_forge_cbor_is_nan(x)) {
        item[n++] = (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | SCE_FORGE_CBOR_INFO_HALF);
        argument = 0x7E00U;
        width = 2;
    } else if (sce_forge_cbor_half_of(x, &half)) {
        item[n++] = (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | SCE_FORGE_CBOR_INFO_HALF);
        argument = half;
        width = 2;
    } else if (sce_forge_cbor_single_of(x, &single)) {
        uint32_t bits;
        memcpy(&bits, &single, sizeof bits);
        item[n++] = (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | SCE_FORGE_CBOR_INFO_SINGLE);
        argument = bits;
        width = 4;
    } else {
        item[n++] = (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | SCE_FORGE_CBOR_INFO_DOUBLE);
        argument = sce_forge_cbor_double_bits(x);
        width = 8;
    }
    while (width > 0U) {
        width--;
        item[n++] = (uint8_t)(argument >> (8U * width));
    }
    return sce_forge_writer_write_bytes(w, item, n);
}

/* Write `false` or `true`. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_bool(sce_forge_writer_t *w, bool value) {
    return sce_forge_writer_write_u8(w, (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | (value ? 21U : 20U)));
}

/* Write a text string of `len` bytes. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_text(sce_forge_writer_t *w, const char *data, size_t len) {
    sce_forge_codec_status_t s = sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_TEXT, (uint64_t)len);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_writer_write_bytes(w, (const uint8_t *)data, len);
}

/* Write a byte string of `len` bytes. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_bytes(sce_forge_writer_t *w, const uint8_t *data,
                                                                  size_t len) {
    sce_forge_codec_status_t s = sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_BYTES, (uint64_t)len);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_writer_write_bytes(w, data, len);
}

/* Write the head of a definite-length array of `items` items: a list of
 * records an entry holds (docs/adr/0086, section 9.2). */
static inline sce_forge_codec_status_t sce_forge_cbor_write_array_head(sce_forge_writer_t *w, uint64_t items) {
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_ARRAY, items);
}

/* Write the head of a definite-length map of `entries` entries. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_map_head(sce_forge_writer_t *w, uint64_t entries) {
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_MAP, entries);
}

/* Write the head of a union's value: a map of one entry keyed by the case's
 * variant name of `len` bytes as a text string (docs/adr/0101 §2.1). The
 * case's record, or the empty map for a case that holds none, is written
 * after it. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_variant(sce_forge_writer_t *w, const char *name,
                                                                    size_t len) {
    sce_forge_codec_status_t s = sce_forge_cbor_write_map_head(w, 1);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_cbor_write_text(w, name, len);
}

/* ── Reading ───────────────────────────────────────────────────────── */

static inline sce_forge_codec_status_t sce_forge_cbor_read_be(sce_forge_cursor_t *c, size_t n, uint64_t *out) {
    const uint8_t *p = sce_forge_cursor_peek(c, n);
    uint64_t v = 0;
    size_t i;
    if (p == NULL) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    for (i = 0; i < n; ++i) {
        v = (v << 8) | p[i];
    }
    (void)sce_forge_cursor_advance(c, n);
    *out = v;
    return SCE_FORGE_CODEC_OK;
}

/* Read one head of the profile as the input wrote it: its major type, its
 * additional information, which tells a float's precision apart, and its
 * argument. What is not well-formed (RFC 8949 Appendix F) is refused: the
 * reserved additional information 28-30, a simple value in the two-byte
 * form below 32 (RFC 8949 §3.3), and a break code where an item stands; so
 * is an indefinite length, which the profile does not hold (docs/adr/0101
 * §3.1). Each is CBOR_MALFORMED. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_item_head(sce_forge_cursor_t *c, uint8_t *major,
                                                                     uint8_t *info, uint64_t *argument) {
    uint64_t initial = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_be(c, 1, &initial);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    *major = (uint8_t)(initial >> 5);
    *info = (uint8_t)(initial & 0x1FU);
    switch (*info) {
    case 24:
        s = sce_forge_cbor_read_be(c, 1, argument);
        break;
    case 25:
        s = sce_forge_cbor_read_be(c, 2, argument);
        break;
    case 26:
        s = sce_forge_cbor_read_be(c, 4, argument);
        break;
    case 27:
        s = sce_forge_cbor_read_be(c, 8, argument);
        break;
    default:
        if (*info >= 24U) {
            return SCE_FORGE_CODEC_CBOR_MALFORMED;
        }
        *argument = *info;
        break;
    }
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (*major == SCE_FORGE_CBOR_MAJOR_SIMPLE && *info == 24U && *argument < 32U) {
        return SCE_FORGE_CODEC_CBOR_MALFORMED;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Read one head: its major type and the value its additional information
 * carries (for a string or a map, the length). An indefinite length, the
 * reserved values 28–30 and a two-byte simple value below 32 are refused. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_head(sce_forge_cursor_t *c, uint8_t *major,
                                                                uint64_t *value) {
    uint8_t info = 0;
    return sce_forge_cbor_read_item_head(c, major, &info, value);
}

/* The argument of a head of `major`; a head of another major type is an
 * item the place does not take. */
static inline sce_forge_codec_status_t sce_forge_cbor_expect(sce_forge_cursor_t *c, uint8_t major, uint64_t *value) {
    uint8_t m = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_head(c, &m, value);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return m == major ? SCE_FORGE_CODEC_OK : SCE_FORGE_CODEC_CBOR_WRONG_TYPE;
}

/* Read an unsigned integer. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_uint(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_UNSIGNED, out);
}

/* Read an unsigned integer an entry of `max` holds. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_uint_upto(sce_forge_cursor_t *c, uint64_t max,
                                                                     uint64_t *out) {
    sce_forge_codec_status_t s = sce_forge_cbor_read_uint(c, out);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return *out > max ? SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE : SCE_FORGE_CODEC_OK;
}

/* Read a signed integer of `least`..`greatest`, the range of its type: major
 * type 0, or major type 1 below zero. A number outside the range is
 * CBOR_OUT_OF_RANGE, another item CBOR_WRONG_TYPE. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_int_within(sce_forge_cursor_t *c, int64_t least,
                                                                      int64_t greatest, int64_t *out) {
    uint8_t major = 0;
    uint8_t info = 0;
    uint64_t argument = 0;
    int64_t value;
    sce_forge_codec_status_t s = sce_forge_cbor_read_item_head(c, &major, &info, &argument);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (major != SCE_FORGE_CBOR_MAJOR_UNSIGNED && major != SCE_FORGE_CBOR_MAJOR_NEGATIVE) {
        return SCE_FORGE_CODEC_CBOR_WRONG_TYPE;
    }
    /* An argument past INT64_MAX is a number past every int64_t. */
    if (argument > (uint64_t)INT64_MAX) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    value = (int64_t)argument;
    if (major == SCE_FORGE_CBOR_MAJOR_NEGATIVE) {
        value = -1 - value;
    }
    if (value < least || value > greatest) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    *out = value;
    return SCE_FORGE_CODEC_OK;
}

/* The value of a half precision float's bits (IEEE 754 binary16), exactly:
 * every half is a single, whose bits are built here and widened. */
static inline double sce_forge_cbor_from_half(uint16_t half) {
    const uint32_t sign = (uint32_t)(half & 0x8000U) << 16;
    uint32_t exponent = (uint32_t)(half >> 10) & 0x1FU;
    uint32_t fraction = (uint32_t)half & 0x3FFU;
    uint32_t bits;
    if (exponent == 0x1FU) {
        /* An infinity, or a NaN, its fraction kept. */
        bits = sign | 0x7F800000U | (fraction << 13);
    } else if (exponent != 0U) {
        /* A normal half: the exponent rebiased from 15 to 127. */
        bits = sign | ((exponent + 112U) << 23) | (fraction << 13);
    } else if (fraction == 0U) {
        bits = sign;
    } else {
        /* A subnormal half, fraction * 2^-24, normalised as a single: 113 is
         * the single's biased exponent of 2^-14, less one per shift. */
        exponent = 113U;
        while ((fraction & 0x400U) == 0U) {
            fraction <<= 1;
            exponent--;
        }
        bits = sign | (exponent << 23) | ((fraction & 0x3FFU) << 13);
    }
    return (double)sce_forge_cbor_single_of_bits(bits);
}

/* Read a float in any of half, single and double precision, as its value: a
 * variation-tolerant read (RFC 8949 §4.1). An integer, a simple value and
 * every other item are CBOR_WRONG_TYPE. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_float(sce_forge_cursor_t *c, double *out) {
    uint8_t major = 0;
    uint8_t info = 0;
    uint64_t argument = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_item_head(c, &major, &info, &argument);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (major != SCE_FORGE_CBOR_MAJOR_SIMPLE) {
        return SCE_FORGE_CODEC_CBOR_WRONG_TYPE;
    }
    switch (info) {
    case SCE_FORGE_CBOR_INFO_HALF:
        *out = sce_forge_cbor_from_half((uint16_t)argument);
        return SCE_FORGE_CODEC_OK;
    case SCE_FORGE_CBOR_INFO_SINGLE:
        *out = (double)sce_forge_cbor_single_of_bits((uint32_t)argument);
        return SCE_FORGE_CODEC_OK;
    case SCE_FORGE_CBOR_INFO_DOUBLE:
        *out = sce_forge_cbor_double_of_bits(argument);
        return SCE_FORGE_CODEC_OK;
    default:
        return SCE_FORGE_CODEC_CBOR_WRONG_TYPE;
    }
}

/* Read a float32: a float in any precision whose value a single holds. A
 * double that no single is exactly is CBOR_OUT_OF_RANGE; a NaN of any
 * precision is the single's quiet NaN. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_float32(sce_forge_cursor_t *c, float *out) {
    double x = 0.0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_float(c, &x);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (sce_forge_cbor_is_nan(x)) {
        *out = sce_forge_cbor_single_of_bits(0x7FC00000U);
        return SCE_FORGE_CODEC_OK;
    }
    return sce_forge_cbor_single_of(x, out) ? SCE_FORGE_CODEC_OK : SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
}

/* Read `false` or `true`. Every other item, `null` and a float among them,
 * is CBOR_WRONG_TYPE. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bool(sce_forge_cursor_t *c, bool *out) {
    uint8_t major = 0;
    uint8_t info = 0;
    uint64_t argument = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_item_head(c, &major, &info, &argument);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (major != SCE_FORGE_CBOR_MAJOR_SIMPLE || (info != 20U && info != 21U)) {
        return SCE_FORGE_CODEC_CBOR_WRONG_TYPE;
    }
    *out = info == 21U;
    return SCE_FORGE_CODEC_OK;
}

/* Copy a string payload of `len` bytes into `dst`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_payload(sce_forge_cursor_t *c, uint64_t len, uint8_t *dst) {
    const uint8_t *p;
    if (len > (uint64_t)sce_forge_cursor_remaining(c)) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    p = sce_forge_cursor_peek(c, (size_t)len);
    if (dst != NULL && len > 0U) {
        memcpy(dst, p, (size_t)len);
    }
    (void)sce_forge_cursor_advance(c, (size_t)len);
    return SCE_FORGE_CODEC_OK;
}

/* Step over a string payload of `len` bytes this node does not hold, judging a
 * text's as a text: a payload past the input is NEED_MORE_BYTES, a text that is
 * not UTF-8 INVALID_UTF8, and any other is CAPACITY_EXCEEDED, which a decode
 * notes and reads on past (`sce_forge_codec_note_capacity`): a storing decode
 * is a store after a parse (docs/adr/0077 §7). */
static inline sce_forge_codec_status_t sce_forge_cbor_pass_payload(sce_forge_cursor_t *c, uint64_t len, bool text) {
    const uint8_t *p;
    if (len > (uint64_t)sce_forge_cursor_remaining(c)) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    p = sce_forge_cursor_peek(c, (size_t)len);
    if (text && !sce_forge_is_valid_utf8(p, (size_t)len)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    (void)sce_forge_cursor_advance(c, (size_t)len);
    return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
}

/* Read a UTF-8 text string into `dst`, which holds `capacity` bytes, its length
 * into `*len`. A text longer than `max_size`, the bound the protocol states
 * (`SIZE_MAX` where it states none), is no frame of the language; one inside
 * it and longer than `capacity` is one this node cannot hold (docs/adr/0077
 * section 7): it is read whole and judged, nothing of it is held (`*len` is
 * 0), and it is CAPACITY_EXCEEDED. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_text(sce_forge_cursor_t *c, size_t max_size, size_t capacity,
                                                                char *dst, size_t *len) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_TEXT, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n > (uint64_t)max_size) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    if (n > (uint64_t)capacity) {
        *len = 0;
        return sce_forge_cbor_pass_payload(c, n, true);
    }
    s = sce_forge_cbor_read_payload(c, n, (uint8_t *)dst);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)dst, (size_t)n)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    *len = (size_t)n;
    return SCE_FORGE_CODEC_OK;
}

/* Read a byte string into `dst`, which holds `capacity` bytes, its length into
 * `*len`: one longer than `max_size` (`SIZE_MAX` for no bound) is no frame of
 * the language, and one inside it and longer than `capacity` is one this node
 * cannot hold, read whole and not held, as for a text. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bytes(sce_forge_cursor_t *c, size_t max_size,
                                                                 size_t capacity, uint8_t *dst, size_t *len) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_BYTES, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n > (uint64_t)max_size) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    if (n > (uint64_t)capacity) {
        *len = 0;
        return sce_forge_cbor_pass_payload(c, n, false);
    }
    s = sce_forge_cbor_read_payload(c, n, dst);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    *len = (size_t)n;
    return SCE_FORGE_CODEC_OK;
}

/* Read a byte string of exactly `length` bytes into `dst`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bytes_exact(sce_forge_cursor_t *c, size_t length,
                                                                       uint8_t *dst) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_BYTES, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n != (uint64_t)length) {
        return SCE_FORGE_CODEC_CBOR_WRONG_LENGTH;
    }
    return sce_forge_cbor_read_payload(c, n, dst);
}

/* Read the head of a definite-length array: how many items follow. An
 * indefinite length is refused, as a map's is. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_array_len(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_ARRAY, out);
}

/* Read a text string where it lies in the input, `*text` its first byte and
 * `*len` its length, copying nothing: the key of a record's field, which a
 * decode compares and does not hold. A text that is not UTF-8 is refused. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_text_in_place(sce_forge_cursor_t *c, const char **text,
                                                                         size_t *len) {
    uint64_t n = 0;
    const uint8_t *p;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_TEXT, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n > (uint64_t)sce_forge_cursor_remaining(c)) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    p = sce_forge_cursor_peek(c, (size_t)n);
    if (!sce_forge_is_valid_utf8(p, (size_t)n)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    (void)sce_forge_cursor_advance(c, (size_t)n);
    *text = (const char *)p;
    *len = (size_t)n;
    return SCE_FORGE_CODEC_OK;
}

/* Read the head of a definite-length map: how many entries follow. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_map_len(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_MAP, out);
}

/* Read the head of a union's value and name its case (docs/adr/0101 §2.1 and
 * §3.3): a map, judged for exactly one entry before its key is read
 * (CBOR_UNION_ENTRIES), whose key is a text string (CBOR_WRONG_TYPE) naming
 * one of the `count` NUL-terminated `names` (CBOR_UNKNOWN_VARIANT). The
 * case's index in `names` is put in `*index`; the case's record, or the
 * empty map, follows. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_variant(sce_forge_cursor_t *c, const char *const *names,
                                                                   size_t count, size_t *index) {
    uint64_t entries = 0;
    const char *text = NULL;
    size_t len = 0;
    size_t i;
    sce_forge_codec_status_t s = sce_forge_cbor_read_map_len(c, &entries);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (entries != 1U) {
        return SCE_FORGE_CODEC_CBOR_UNION_ENTRIES;
    }
    s = sce_forge_cbor_read_text_in_place(c, &text, &len);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    for (i = 0; i < count; ++i) {
        if (sce_text_length(names[i]) == len && (len == 0U || memcmp(names[i], text, len) == 0)) {
            *index = i;
            return SCE_FORGE_CODEC_OK;
        }
    }
    return SCE_FORGE_CODEC_CBOR_UNKNOWN_VARIANT;
}

/* Skip one item — the value of a key the codec does not declare — and
 * everything nested in it, refusing one nested deeper than
 * SCE_FORGE_CBOR_MAX_SKIP_DEPTH and one the profile does not hold (an
 * indefinite length at any depth). `pending[d]` counts what is still to be
 * read at nesting depth `d` — items of an array or a tag, entries of a map,
 * whose key and value `in_value[d]` tells apart — so a map's count is used
 * as written, never doubled. An item is refused when it would be read at
 * depth SCE_FORGE_CBOR_MAX_SKIP_DEPTH, the depth at which the Rust
 * runtime's recursive skip refuses it, and every other refusal is the one
 * the recursion meets first on the same input. */
static inline sce_forge_codec_status_t sce_forge_cbor_skip(sce_forge_cursor_t *c) {
    uint64_t pending[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    bool is_map[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    bool in_value[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    size_t depth = 0;
    pending[0] = 1;
    is_map[0] = false;
    in_value[0] = false;
    for (;;) {
        uint8_t major = 0;
        uint64_t value = 0;
        sce_forge_codec_status_t s;
        if (pending[depth] == 0U) {
            if (depth == 0U) {
                return SCE_FORGE_CODEC_OK;
            }
            depth--;
            continue;
        }
        if (depth >= (size_t)SCE_FORGE_CBOR_MAX_SKIP_DEPTH) {
            return SCE_FORGE_CODEC_CBOR_TOO_DEEP;
        }
        /* An entry of a map is spent once its value is read. */
        if (is_map[depth] && !in_value[depth]) {
            in_value[depth] = true;
        } else {
            in_value[depth] = false;
            pending[depth]--;
        }
        s = sce_forge_cbor_read_head(c, &major, &value);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
        switch (major) {
        /* Unsigned, negative, simple / float: the head is the whole item. */
        case 0:
        case 1:
        case 7:
            break;
        case 2:
        case 3:
            s = sce_forge_cbor_read_payload(c, value, NULL);
            if (s != SCE_FORGE_CODEC_OK) {
                return s;
            }
            break;
        case 4:
        case 5:
        /* A tag: its one enclosed item follows. */
        case 6:
            depth++;
            pending[depth] = major == 6U ? 1U : value;
            is_map[depth] = major == 5U;
            in_value[depth] = false;
            break;
        default:
            return SCE_FORGE_CODEC_CBOR_MALFORMED;
        }
    }
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_CBOR_H */
