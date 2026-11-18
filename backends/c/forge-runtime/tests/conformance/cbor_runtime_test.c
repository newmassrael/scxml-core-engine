/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/cbor.h — the properties the Rust runtime's own tests pin
 * (backends/rust/forge-runtime/src/cbor.rs): shortest heads and floats on
 * write, any valid head length and float precision on read, a signed integer
 * at every head boundary, each refusal by its kind (docs/adr/0101 §3.3), a
 * union's one-entry map, and a skip that takes an unknown value whole and
 * refuses one too deep or of indefinite length — here walked without
 * recursion, so the depth at which it refuses is checked against the
 * recursive one's. What a generated codec does with these is the
 * conformance harness's (codec_cbor_map), which this file does not repeat.
 */

#include "sce/forge/cbor.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

static int written_is(const uint8_t *got, size_t got_len, const uint8_t *want, size_t want_len) {
    return got_len == want_len && memcmp(got, want, want_len) == 0;
}

static void every_head_is_written_in_its_shortest_form(void) {
    /* RFC 8949 Appendix A: 0, 23, 24, 255, 256, 65535, 65536, 2^32. */
    static const struct {
        uint64_t value;
        uint8_t want[9];
        size_t len;
    } cases[] = {
        {0, {0x00}, 1},
        {23, {0x17}, 1},
        {24, {0x18, 0x18}, 2},
        {255, {0x18, 0xff}, 2},
        {256, {0x19, 0x01, 0x00}, 3},
        {65535, {0x19, 0xff, 0xff}, 3},
        {65536, {0x1a, 0x00, 0x01, 0x00, 0x00}, 5},
        {(uint64_t)1 << 32, {0x1b, 0, 0, 0, 1, 0, 0, 0, 0}, 9},
    };

    size_t i;
    for (i = 0; i < sizeof cases / sizeof cases[0]; ++i) {
        uint8_t buf[16];
        sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
        CHECK(sce_forge_cbor_write_uint(&w, cases[i].value) == SCE_FORGE_CODEC_OK, "write_uint");
        CHECK(written_is(buf, sce_forge_writer_position(&w), cases[i].want, cases[i].len), "shortest head");
    }
    {
        uint8_t buf[16];
        static const uint8_t want[] = {0x61, 'a', 0x42, 1, 2, 0xf5, 0xa2};
        static const uint8_t two[] = {1, 2};
        sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
        CHECK(sce_forge_cbor_write_text(&w, "a", 1) == SCE_FORGE_CODEC_OK, "text");
        CHECK(sce_forge_cbor_write_bytes(&w, two, 2) == SCE_FORGE_CODEC_OK, "bytes");
        CHECK(sce_forge_cbor_write_bool(&w, true) == SCE_FORGE_CODEC_OK, "true");
        CHECK(sce_forge_cbor_write_map_head(&w, 2) == SCE_FORGE_CODEC_OK, "map head");
        CHECK(written_is(buf, sce_forge_writer_position(&w), want, sizeof want), "text, bytes, true, map head");
    }
}

static void an_array_head_and_a_key_in_place_are_read_and_written(void) {
    uint8_t buf[8];
    static const uint8_t want[] = {0x82, 0x98, 0x18};
    static const uint8_t three[] = {0x83};
    static const uint8_t wide[] = {0x98, 0x18};
    static const uint8_t indefinite[] = {0x9f};
    static const uint8_t map[] = {0xa1};
    static const uint8_t key[] = {0x63, 's', 'e', 'q', 0x01};
    static const uint8_t not_utf8[] = {0x61, 0xff};
    uint64_t v = 0;
    const char *text = NULL;
    size_t len = 0;
    sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
    sce_forge_cursor_t c;
    CHECK(sce_forge_cbor_write_array_head(&w, 2) == SCE_FORGE_CODEC_OK, "array head");
    CHECK(sce_forge_cbor_write_array_head(&w, 24) == SCE_FORGE_CODEC_OK, "array head of 24");
    CHECK(written_is(buf, sce_forge_writer_position(&w), want, sizeof want), "array heads, shortest");
    c = sce_forge_cursor_init(three, sizeof three);
    CHECK(sce_forge_cbor_read_array_len(&c, &v) == SCE_FORGE_CODEC_OK && v == 3, "an array of three");
    c = sce_forge_cursor_init(wide, sizeof wide);
    CHECK(sce_forge_cbor_read_array_len(&c, &v) == SCE_FORGE_CODEC_OK && v == 24, "an array of 24");
    /* An indefinite-length array, and a map where an array stands. */
    c = sce_forge_cursor_init(indefinite, sizeof indefinite);
    CHECK(sce_forge_cbor_read_array_len(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "indefinite-length array");
    c = sce_forge_cursor_init(map, sizeof map);
    CHECK(sce_forge_cbor_read_array_len(&c, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "a map is no array");
    /* A key is read where it lies, and the cursor lands after it. */
    c = sce_forge_cursor_init(key, sizeof key);
    CHECK(sce_forge_cbor_read_text_in_place(&c, &text, &len) == SCE_FORGE_CODEC_OK && len == 3U &&
              memcmp(text, "seq", 3U) == 0,
          "a key in place");
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_OK && v == 1, "the read lands after the key");
    c = sce_forge_cursor_init(not_utf8, sizeof not_utf8);
    CHECK(sce_forge_cbor_read_text_in_place(&c, &text, &len) == SCE_FORGE_CODEC_INVALID_UTF8,
          "a key that is not UTF-8");
    c = sce_forge_cursor_init(key, 2U);
    CHECK(sce_forge_cbor_read_text_in_place(&c, &text, &len) == SCE_FORGE_CODEC_NEED_MORE_BYTES, "a short key");
}

static void a_head_is_read_in_any_valid_length(void) {
    /* 1 written in its 2-byte form still reads as 1. */
    static const uint8_t two[] = {0x18, 0x01};
    static const uint8_t nine[] = {0x1b, 0, 0, 0, 1, 0, 0, 0, 0};
    uint64_t v = 0;
    sce_forge_cursor_t c = sce_forge_cursor_init(two, sizeof two);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_OK && v == 1, "1 in a 2-byte head");
    c = sce_forge_cursor_init(nine, sizeof nine);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_OK && v == (uint64_t)1 << 32, "2^32");
}

static void what_the_codec_cannot_read_is_refused(void) {
    static const uint8_t indefinite[] = {0xbf};
    static const uint8_t reserved[] = {0x1c};
    static const uint8_t text[] = {0x61, 'a'};
    static const uint8_t wide[] = {0x19, 0x01, 0x00};
    static const uint8_t short_bytes[] = {0x42, 1, 2};
    static const uint8_t long_text[] = {0x62, 'a', 'b'};
    static const uint8_t not_utf8[] = {0x61, 0xff};
    uint64_t v = 0;
    uint8_t dst[16];
    char txt[16];
    size_t len = 0;
    sce_forge_cursor_t c;

    c = sce_forge_cursor_init(indefinite, sizeof indefinite);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "indefinite length");
    c = sce_forge_cursor_init(reserved, sizeof reserved);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "reserved information");
    c = sce_forge_cursor_init(text, sizeof text);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "another major type");
    c = sce_forge_cursor_init(indefinite, sizeof indefinite);
    CHECK(sce_forge_cbor_read_map_len(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "indefinite-length map");
    c = sce_forge_cursor_init(wide, sizeof wide);
    CHECK(sce_forge_cbor_read_uint_upto(&c, 255, &v) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE, "256 in a uint8");
    c = sce_forge_cursor_init(short_bytes, sizeof short_bytes);
    CHECK(sce_forge_cbor_read_bytes_exact(&c, 16, dst) == SCE_FORGE_CODEC_CBOR_WRONG_LENGTH,
          "two bytes where sixteen are declared");
    c = sce_forge_cursor_init(long_text, sizeof long_text);
    CHECK(sce_forge_cbor_read_text(&c, 1, sizeof txt, txt, &len) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE,
          "a text past its bound");
    c = sce_forge_cursor_init(long_text, sizeof long_text);
    CHECK(sce_forge_cbor_read_text(&c, SIZE_MAX, 1, txt, &len) == SCE_FORGE_CODEC_CAPACITY_EXCEEDED,
          "a text inside its bound and past its capacity");
    /* It is read whole and not held (docs/adr/0077 §7): the cursor stands after
     * it, so a decode reads the map on. */
    CHECK(sce_forge_cursor_remaining(&c) == 0U && len == 0U, "a text past its capacity is stepped over");
    c = sce_forge_cursor_init(not_utf8, sizeof not_utf8);
    CHECK(sce_forge_cbor_read_text(&c, sizeof txt, sizeof txt, txt, &len) == SCE_FORGE_CODEC_INVALID_UTF8,
          "a text that is not UTF-8");
    /* A text past the capacity is still judged: one that is not UTF-8 is that,
     * and one past the input needs more bytes. */
    c = sce_forge_cursor_init(not_utf8, sizeof not_utf8);
    CHECK(sce_forge_cbor_read_text(&c, SIZE_MAX, 0, txt, &len) == SCE_FORGE_CODEC_INVALID_UTF8,
          "a text past its capacity that is not UTF-8");
    c = sce_forge_cursor_init(long_text, sizeof long_text - 1U);
    CHECK(sce_forge_cbor_read_text(&c, SIZE_MAX, 1, txt, &len) == SCE_FORGE_CODEC_NEED_MORE_BYTES,
          "a text past its capacity and past the input");
}

static void an_unknown_value_is_skipped_whole_and_a_deep_one_refused(void) {
    /* {1: [2, {3: h'00'}]} then 7: the skip lands on the 7. */
    static const uint8_t nested[] = {0xa1, 0x01, 0x82, 0x02, 0xa1, 0x03, 0x41, 0x00, 0x07};
    uint8_t deep[21];
    uint8_t edge[17];
    uint64_t v = 0;
    sce_forge_cursor_t c = sce_forge_cursor_init(nested, sizeof nested);
    CHECK(sce_forge_cbor_skip(&c) == SCE_FORGE_CODEC_OK, "a nested value is skipped");
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_OK && v == 7, "the skip lands on the next item");

    memset(deep, 0x81, 20);
    deep[20] = 0x00;
    c = sce_forge_cursor_init(deep, sizeof deep);
    CHECK(sce_forge_cbor_skip(&c) == SCE_FORGE_CODEC_CBOR_TOO_DEEP, "twenty nested arrays");

    /* The edge the recursion draws: fifteen nested one-item arrays around
     * an empty one is an item at depth 15 and is skipped; sixteen around a
     * 0 puts the 0 at depth 16 and is refused. */
    memset(edge, 0x81, 15);
    edge[15] = 0x80;
    c = sce_forge_cursor_init(edge, 16);
    CHECK(sce_forge_cbor_skip(&c) == SCE_FORGE_CODEC_OK, "an empty array at depth 15");
    memset(edge, 0x81, 16);
    edge[16] = 0x00;
    c = sce_forge_cursor_init(edge, 17);
    CHECK(sce_forge_cbor_skip(&c) == SCE_FORGE_CODEC_CBOR_TOO_DEEP, "an item at depth 16");
}

/* A byte list of at most nine bytes, as a case states it. */
typedef struct {
    uint8_t bytes[9];
    size_t len;
} item_t;

static sce_forge_cursor_t cursor_of(const item_t *item) {
    return sce_forge_cursor_init(item->bytes, item->len);
}

static void a_skipped_value_of_indefinite_length_is_refused_at_any_depth(void) {
    static const item_t items[] = {
        {{0x9f, 0x01, 0xff}, 3},
        {{0xbf, 0x01, 0x02, 0xff}, 4},
        {{0x7f, 0x61, 'a', 0xff}, 4},
        {{0x5f, 0x41, 0x00, 0xff}, 4},
        /* Nested one and two levels down. */
        {{0x81, 0x9f, 0xff}, 3},
        {{0xa1, 0x01, 0x81, 0x7f, 0xff}, 5},
    };
    size_t i;
    for (i = 0; i < sizeof items / sizeof items[0]; ++i) {
        sce_forge_cursor_t c = cursor_of(&items[i]);
        CHECK(sce_forge_cbor_skip(&c) == SCE_FORGE_CODEC_CBOR_MALFORMED, "a skipped indefinite length");
    }
}

static void a_signed_integer_is_written_in_its_shortest_head_and_read_back(void) {
    /* Each boundary of a head's length, on both sides of zero: the argument
     * of a negative number is -1 - n, so -24 is the last of one byte and -25
     * the first of two. */
    static const struct {
        int64_t value;
        item_t item;
    } cases[] = {
        {0, {{0x00}, 1}},
        {23, {{0x17}, 1}},
        {24, {{0x18, 0x18}, 2}},
        {-1, {{0x20}, 1}},
        {-24, {{0x37}, 1}},
        {-25, {{0x38, 0x18}, 2}},
        {-256, {{0x38, 0xff}, 2}},
        {-257, {{0x39, 0x01, 0x00}, 3}},
        {-500, {{0x39, 0x01, 0xf3}, 3}},
        {-65536, {{0x39, 0xff, 0xff}, 3}},
        {-65537, {{0x3a, 0x00, 0x01, 0x00, 0x00}, 5}},
        {-(INT64_C(1) << 32), {{0x3a, 0xff, 0xff, 0xff, 0xff}, 5}},
        {-(INT64_C(1) << 32) - 1, {{0x3b, 0, 0, 0, 1, 0, 0, 0, 0}, 9}},
        {INT64_MIN, {{0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}, 9}},
        {INT64_MAX, {{0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}, 9}},
        {(INT64_C(1) << 32) - 1, {{0x1a, 0xff, 0xff, 0xff, 0xff}, 5}},
    };

    size_t i;
    for (i = 0; i < sizeof cases / sizeof cases[0]; ++i) {
        uint8_t buf[16];
        int64_t v = 0;
        sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
        sce_forge_cursor_t c = cursor_of(&cases[i].item);
        CHECK(sce_forge_cbor_write_int(&w, cases[i].value) == SCE_FORGE_CODEC_OK, "write_int");
        CHECK(written_is(buf, sce_forge_writer_position(&w), cases[i].item.bytes, cases[i].item.len),
              "a signed integer's shortest head");
        CHECK(sce_forge_cbor_read_int_within(&c, INT64_MIN, INT64_MAX, &v) == SCE_FORGE_CODEC_OK && v == cases[i].value,
              "a signed integer read back");
    }
}

static void a_signed_integer_is_read_in_any_head_and_held_to_its_range(void) {
    static const item_t minus_one_wide = {{0x38, 0x00}, 2};
    static const item_t least_int8 = {{0x38, 0x7f}, 2};
    static const item_t greatest_int8 = {{0x18, 0x7f}, 2};
    static const item_t below_int8 = {{0x38, 0x80}, 2};
    static const item_t past_int8 = {{0x18, 0x80}, 2};
    static const item_t below_int64 = {{0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0}, 9};
    static const item_t past_int64 = {{0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0}, 9};
    static const item_t half = {{0xf9, 0x3c, 0x00}, 3};
    static const item_t text = {{0x61, 'a'}, 2};
    int64_t v = 0;
    sce_forge_cursor_t c = cursor_of(&minus_one_wide);
    CHECK(sce_forge_cbor_read_int_within(&c, -128, 127, &v) == SCE_FORGE_CODEC_OK && v == -1, "-1 in two bytes");
    c = cursor_of(&least_int8);
    CHECK(sce_forge_cbor_read_int_within(&c, -128, 127, &v) == SCE_FORGE_CODEC_OK && v == -128, "an int8's least");
    c = cursor_of(&greatest_int8);
    CHECK(sce_forge_cbor_read_int_within(&c, -128, 127, &v) == SCE_FORGE_CODEC_OK && v == 127, "an int8's greatest");
    c = cursor_of(&below_int8);
    CHECK(sce_forge_cbor_read_int_within(&c, -128, 127, &v) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE, "-129 in an int8");
    c = cursor_of(&past_int8);
    CHECK(sce_forge_cbor_read_int_within(&c, -128, 127, &v) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE, "128 in an int8");
    c = cursor_of(&below_int64);
    CHECK(sce_forge_cbor_read_int_within(&c, INT64_MIN, INT64_MAX, &v) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE,
          "below an int64's least");
    c = cursor_of(&past_int64);
    CHECK(sce_forge_cbor_read_int_within(&c, INT64_MIN, INT64_MAX, &v) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE,
          "past an int64's greatest");
    c = cursor_of(&half);
    CHECK(sce_forge_cbor_read_int_within(&c, INT64_MIN, INT64_MAX, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE,
          "a float is no integer");
    c = cursor_of(&text);
    CHECK(sce_forge_cbor_read_int_within(&c, INT64_MIN, INT64_MAX, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE,
          "a text is no integer");
}

static void a_float_is_written_in_the_shortest_precision_that_holds_it_and_read_back(void) {
    /* RFC 8949 Appendix A, and the edges of each precision. */
    const struct {
        double value;
        item_t item;
    } cases[] = {
        {0.0, {{0xf9, 0x00, 0x00}, 3}},
        {-0.0, {{0xf9, 0x80, 0x00}, 3}},
        {1.0, {{0xf9, 0x3c, 0x00}, 3}},
        {1.1, {{0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a}, 9}},
        {1.5, {{0xf9, 0x3e, 0x00}, 3}},
        /* The greatest half, and one past it in a half's spacing there,
         * which a single holds. */
        {65504.0, {{0xf9, 0x7b, 0xff}, 3}},
        {65505.0, {{0xfa, 0x47, 0x7f, 0xe1, 0x00}, 5}},
        {100000.0, {{0xfa, 0x47, 0xc3, 0x50, 0x00}, 5}},
        {(double)FLT_MAX, {{0xfa, 0x7f, 0x7f, 0xff, 0xff}, 5}},
        {(double)0.1F, {{0xfa, 0x3d, 0xcc, 0xcc, 0xcd}, 5}},
        {1.0e300, {{0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c}, 9}},
        /* The least half, a subnormal; the greatest half subnormal; the
         * least half normal; half the least half, which only a single holds. */
        {0x1p-24, {{0xf9, 0x00, 0x01}, 3}},
        {1023.0 * 0x1p-24, {{0xf9, 0x03, 0xff}, 3}},
        {0x1p-14, {{0xf9, 0x04, 0x00}, 3}},
        {0x1p-25, {{0xfa, 0x33, 0x00, 0x00, 0x00}, 5}},
        /* One bit below a single's last: only a double holds it. */
        {1.0 + 0x1p-24, {{0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00}, 9}},
        {-4.0, {{0xf9, 0xc4, 0x00}, 3}},
        {-4.1, {{0xfb, 0xc0, 0x10, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66}, 9}},
        {sce_forge_cbor_double_of_bits(UINT64_C(0x7FF0000000000000)), {{0xf9, 0x7c, 0x00}, 3}},
        {sce_forge_cbor_double_of_bits(UINT64_C(0xFFF0000000000000)), {{0xf9, 0xfc, 0x00}, 3}},
    };

    /* Every NaN is the one NaN, whatever its sign and payload. */
    const uint64_t nans[] = {UINT64_C(0x7FF8000000000000), UINT64_C(0x7FF0000000000001), UINT64_C(0xFFF8000000000000)};
    static const uint8_t one_nan[] = {0xf9, 0x7e, 0x00};
    size_t i;
    for (i = 0; i < sizeof cases / sizeof cases[0]; ++i) {
        uint8_t buf[16];
        double v = 0.0;
        sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
        sce_forge_cursor_t c = cursor_of(&cases[i].item);
        CHECK(sce_forge_cbor_write_float(&w, cases[i].value) == SCE_FORGE_CODEC_OK, "write_float");
        CHECK(written_is(buf, sce_forge_writer_position(&w), cases[i].item.bytes, cases[i].item.len),
              "a float's shortest precision");
        CHECK(sce_forge_cbor_read_float(&c, &v) == SCE_FORGE_CODEC_OK &&
                  sce_forge_cbor_double_bits(v) == sce_forge_cbor_double_bits(cases[i].value),
              "a float read back");
    }
    for (i = 0; i < sizeof nans / sizeof nans[0]; ++i) {
        uint8_t buf[16];
        sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
        CHECK(sce_forge_cbor_write_float(&w, sce_forge_cbor_double_of_bits(nans[i])) == SCE_FORGE_CODEC_OK,
              "write a NaN");
        CHECK(written_is(buf, sce_forge_writer_position(&w), one_nan, sizeof one_nan), "the one NaN");
    }
}

static void a_float_is_read_in_any_precision_that_holds_it(void) {
    /* 1.5 and a NaN as a half, a single and a double. */
    static const item_t one_and_a_half[] = {
        {{0xf9, 0x3e, 0x00}, 3},
        {{0xfa, 0x3f, 0xc0, 0x00, 0x00}, 5},
        {{0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0}, 9},
    };
    static const item_t nan[] = {
        {{0xf9, 0x7e, 0x00}, 3},
        {{0xfa, 0x7f, 0xc0, 0x00, 0x00}, 5},
        {{0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0}, 9},
    };
    /* 0.1 as a double is no single; an integer, a truth value, null and a
     * negative integer are no float. */
    static const item_t tenth = {{0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a}, 9};
    static const item_t no_float[] = {{{0x01}, 1}, {{0xf5}, 1}, {{0xf6}, 1}, {{0x20}, 1}};
    double wide = 0.0;
    float narrow = 0.0F;
    sce_forge_cursor_t c;
    size_t i;
    for (i = 0; i < 3U; ++i) {
        c = cursor_of(&one_and_a_half[i]);
        CHECK(sce_forge_cbor_read_float(&c, &wide) == SCE_FORGE_CODEC_OK && wide == 1.5, "1.5 as a float64");
        c = cursor_of(&one_and_a_half[i]);
        CHECK(sce_forge_cbor_read_float32(&c, &narrow) == SCE_FORGE_CODEC_OK && narrow == 1.5F, "1.5 as a float32");
        c = cursor_of(&nan[i]);
        CHECK(sce_forge_cbor_read_float(&c, &wide) == SCE_FORGE_CODEC_OK && sce_forge_cbor_is_nan(wide),
              "a NaN as a float64");
        c = cursor_of(&nan[i]);
        CHECK(sce_forge_cbor_read_float32(&c, &narrow) == SCE_FORGE_CODEC_OK && sce_forge_cbor_is_nan((double)narrow),
              "a NaN as a float32");
    }
    c = cursor_of(&tenth);
    CHECK(sce_forge_cbor_read_float(&c, &wide) == SCE_FORGE_CODEC_OK && wide == 0.1, "0.1 as a float64");
    c = cursor_of(&tenth);
    CHECK(sce_forge_cbor_read_float32(&c, &narrow) == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE, "0.1 as a float32");
    for (i = 0; i < sizeof no_float / sizeof no_float[0]; ++i) {
        c = cursor_of(&no_float[i]);
        CHECK(sce_forge_cbor_read_float(&c, &wide) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "no float");
    }
}

static void an_item_that_is_not_well_formed_or_not_in_the_profile_is_malformed(void) {
    /* An indefinite text and byte string, in chunks, and a break code where
     * an item stands; a simple value in the two-byte form below 32, which is
     * not well-formed (RFC 8949 §3.3); the reserved values 28-30. */
    static const item_t chunked_text = {{0x7f, 0x61, 'a', 0x61, 'b', 0xff}, 6};
    static const item_t chunked_bytes = {{0x5f, 0x41, 0x00, 0xff}, 4};
    static const item_t stop = {{0xff}, 1};
    static const item_t simple = {{0xf8, 0x14}, 2};
    static const item_t reserved[] = {{{0x1c}, 1}, {{0x1d}, 1}, {{0x1e}, 1}};
    char txt[8];
    uint8_t dst[8];
    size_t len = 0;
    uint64_t v = 0;
    uint8_t major = 0;
    bool b = false;
    size_t i;
    sce_forge_cursor_t c = cursor_of(&chunked_text);
    CHECK(sce_forge_cbor_read_text(&c, sizeof txt, sizeof txt, txt, &len) == SCE_FORGE_CODEC_CBOR_MALFORMED,
          "an indefinite text");
    c = cursor_of(&chunked_bytes);
    CHECK(sce_forge_cbor_read_bytes(&c, sizeof dst, sizeof dst, dst, &len) == SCE_FORGE_CODEC_CBOR_MALFORMED,
          "an indefinite byte string");
    c = cursor_of(&stop);
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "a break code");
    c = cursor_of(&simple);
    CHECK(sce_forge_cbor_read_bool(&c, &b) == SCE_FORGE_CODEC_CBOR_MALFORMED, "a two-byte simple value below 32");
    for (i = 0; i < sizeof reserved / sizeof reserved[0]; ++i) {
        c = cursor_of(&reserved[i]);
        CHECK(sce_forge_cbor_read_head(&c, &major, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "a reserved head");
    }
}

static void an_item_the_place_does_not_take_is_of_the_wrong_type(void) {
    /* A negative integer where an unsigned stands, null, a bignum (a tagged
     * item); null, a float whose bits are a truth value's number and an
     * integer where a bool stands. */
    static const item_t no_uint[] = {{{0x20}, 1}, {{0xf6}, 1}, {{0xc2, 0x41, 0x01}, 3}};
    static const item_t no_bool[] = {{{0xf6}, 1}, {{0xf9, 0x00, 0x14}, 3}, {{0x01}, 1}};
    static const item_t uint_key = {{0x01}, 1};
    static const item_t array = {{0x81, 0x01}, 2};
    uint64_t v = 0;
    bool b = false;
    const char *text = NULL;
    size_t len = 0;
    sce_forge_cursor_t c;
    size_t i;
    for (i = 0; i < 3U; ++i) {
        c = cursor_of(&no_uint[i]);
        CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "no unsigned integer");
        c = cursor_of(&no_bool[i]);
        CHECK(sce_forge_cbor_read_bool(&c, &b) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "no bool");
    }
    c = cursor_of(&uint_key);
    CHECK(sce_forge_cbor_read_text_in_place(&c, &text, &len) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE,
          "an unsigned key where a text key stands");
    c = cursor_of(&array);
    CHECK(sce_forge_cbor_read_map_len(&c, &v) == SCE_FORGE_CODEC_CBOR_WRONG_TYPE, "an array where a map stands");
}

static void a_union_is_a_map_of_one_entry_named_by_its_case(void) {
    /* docs/adr/0101 §2.8: `cash` of `union:Payment` is a1 64 63 61 73 68 a0. */
    static const char *const payment[] = {"card", "cash", "voucher"};
    static const item_t cash = {{0xa1, 0x64, 'c', 'a', 's', 'h', 0xa0}, 7};
    static const item_t refused[] = {
        /* No entry, and two: the count is judged before any key is read. */
        {{0xa0}, 1},
        {{0xa2}, 1},
        /* A case no variant names. */
        {{0xa1, 0x64, 'c', 'o', 'i', 'n', 0xa0}, 7},
        /* A key that is no text, and an array where the union's map stands. */
        {{0xa1, 0x01, 0xa0}, 3},
        {{0x81, 0xa0}, 2},
    };
    static const sce_forge_codec_status_t why[] = {
        SCE_FORGE_CODEC_CBOR_UNION_ENTRIES, SCE_FORGE_CODEC_CBOR_UNION_ENTRIES, SCE_FORGE_CODEC_CBOR_UNKNOWN_VARIANT,
        SCE_FORGE_CODEC_CBOR_WRONG_TYPE,    SCE_FORGE_CODEC_CBOR_WRONG_TYPE,
    };
    uint8_t buf[16];
    size_t index = 0;
    uint64_t entries = 1;
    size_t i;
    sce_forge_writer_t w = sce_forge_writer_init_buf(buf, sizeof buf);
    sce_forge_cursor_t c = cursor_of(&cash);
    CHECK(sce_forge_cbor_write_variant(&w, "cash", 4) == SCE_FORGE_CODEC_OK, "write_variant");
    CHECK(sce_forge_cbor_write_map_head(&w, 0) == SCE_FORGE_CODEC_OK, "the empty case's map");
    CHECK(written_is(buf, sce_forge_writer_position(&w), cash.bytes, cash.len), "a union's one-entry map");
    CHECK(sce_forge_cbor_read_variant(&c, payment, 3, &index) == SCE_FORGE_CODEC_OK && index == 1U, "cash is case 1");
    CHECK(sce_forge_cbor_read_map_len(&c, &entries) == SCE_FORGE_CODEC_OK && entries == 0U, "cash's empty map");
    for (i = 0; i < sizeof refused / sizeof refused[0]; ++i) {
        c = cursor_of(&refused[i]);
        CHECK(sce_forge_cbor_read_variant(&c, payment, 3, &index) == why[i], "a union refused by its kind");
    }
}

int main(void) {
    every_head_is_written_in_its_shortest_form();
    a_head_is_read_in_any_valid_length();
    an_array_head_and_a_key_in_place_are_read_and_written();
    what_the_codec_cannot_read_is_refused();
    an_unknown_value_is_skipped_whole_and_a_deep_one_refused();
    a_skipped_value_of_indefinite_length_is_refused_at_any_depth();
    a_signed_integer_is_written_in_its_shortest_head_and_read_back();
    a_signed_integer_is_read_in_any_head_and_held_to_its_range();
    a_float_is_written_in_the_shortest_precision_that_holds_it_and_read_back();
    a_float_is_read_in_any_precision_that_holds_it();
    an_item_that_is_not_well_formed_or_not_in_the_profile_is_malformed();
    an_item_the_place_does_not_take_is_of_the_wrong_type();
    a_union_is_a_map_of_one_entry_named_by_its_case();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    puts("cbor_runtime_test (C11): all checks passed");
    return 0;
}
