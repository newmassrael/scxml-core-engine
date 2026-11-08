/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/cbor.h — the same four properties the Rust runtime's own tests
 * pin (backends/rust/forge-runtime/src/cbor.rs): shortest heads on write,
 * any valid head length on read, refusal of what an entry cannot be, and a
 * skip that takes an unknown value whole and refuses one too deep — here
 * walked without recursion, so the depth at which it refuses is checked
 * against the recursive one's. What a generated codec does with these is
 * the conformance harness's (codec_cbor_map), which this file does not
 * repeat.
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
    CHECK(sce_forge_cbor_read_array_len(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "a map is no array");
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
    CHECK(sce_forge_cbor_read_uint(&c, &v) == SCE_FORGE_CODEC_CBOR_MALFORMED, "another major type");
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

int main(void) {
    every_head_is_written_in_its_shortest_form();
    a_head_is_read_in_any_valid_length();
    an_array_head_and_a_key_in_place_are_read_and_written();
    what_the_codec_cannot_read_is_refused();
    an_unknown_value_is_skipped_whole_and_a_deep_one_refused();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    puts("cbor_runtime_test (C11): all checks passed");
    return 0;
}
