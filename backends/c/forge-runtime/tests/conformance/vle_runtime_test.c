/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/codec.h — the cursor's VLE readers on their own. The wire of a
 * `vle_u<N>` field is a ZInt64 at every width, as `zenoh` and `zenoh-pico`
 * read it, and the type bounds the value: a narrow read accepts every encoding
 * of a value it holds, the padded ones included, and refuses a value past its
 * width with SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW, never a longer encoding. What
 * a generated codec does with them is the conformance harness's
 * (codec_vle_zint_u16, _u32 and _u64), which this file does not repeat.
 */

#include "sce/forge/codec.h"

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

static void a_u16_reads_each_encoding_of_a_value_it_holds_alike(void) {
    static const struct {
        uint8_t wire[9];
        size_t len;
        uint16_t value;
    } cases[] = {
        {{0x81, 0x80, 0x80, 0x00}, 4, 1}, {{0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 9, 1},
        {{0xFF, 0x7F}, 2, 0x3FFF},        {{0xFF, 0xFF, 0x80, 0x00}, 4, 0x3FFF},
        {{0x80, 0x80, 0x01}, 3, 0x4000},  {{0x80, 0x80, 0x81, 0x80, 0x00}, 5, 0x4000},
        {{0xFF, 0xFF, 0x03}, 3, 0xFFFF},  {{0xFF, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 9, 0xFFFF},
    };

    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        sce_forge_cursor_t cursor = sce_forge_cursor_init(cases[i].wire, cases[i].len);
        uint16_t value = 0;
        CHECK(sce_forge_cursor_read_vle_u16(&cursor, &value) == SCE_FORGE_CODEC_OK,
              "a u16 reads every encoding of a value it holds");
        CHECK(value == cases[i].value, "the value read is the value encoded");
        CHECK(sce_forge_cursor_remaining(&cursor) == 0, "the read takes the whole ZInt");
    }
}

static void a_u32_and_a_u64_read_a_padded_encoding_of_a_value_they_hold(void) {
    static const uint8_t nine[] = {0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00};
    sce_forge_cursor_t u32 = sce_forge_cursor_init(nine, sizeof(nine));
    uint32_t value32 = 0;
    CHECK(sce_forge_cursor_read_vle_u32(&u32, &value32) == SCE_FORGE_CODEC_OK && value32 == 1u,
          "a u32 reads 1 in nine bytes");
    sce_forge_cursor_t u64 = sce_forge_cursor_init(nine, sizeof(nine));
    uint64_t value64 = 0;
    CHECK(sce_forge_cursor_read_vle_u64(&u64, &value64) == SCE_FORGE_CODEC_OK && value64 == 1u,
          "a u64 reads 1 in nine bytes");
    static const uint8_t padded_max[] = {0xFF, 0xFF, 0xFF, 0xFF, 0x8F, 0x80, 0x80, 0x80, 0x00};
    sce_forge_cursor_t max32 = sce_forge_cursor_init(padded_max, sizeof(padded_max));
    CHECK(sce_forge_cursor_read_vle_u32(&max32, &value32) == SCE_FORGE_CODEC_OK && value32 == UINT32_MAX,
          "a u32 reads its largest value padded to nine bytes");
    static const uint8_t all_ones[] = {0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF};
    sce_forge_cursor_t max64 = sce_forge_cursor_init(all_ones, sizeof(all_ones));
    CHECK(sce_forge_cursor_read_vle_u64(&max64, &value64) == SCE_FORGE_CODEC_OK && value64 == UINT64_MAX,
          "a u64 reads its largest value in nine bytes");
}

static void a_narrow_read_refuses_the_first_value_past_its_width(void) {
    static const uint8_t past16[] = {0x80, 0x80, 0x04};
    sce_forge_cursor_t u16 = sce_forge_cursor_init(past16, sizeof(past16));
    uint16_t value16 = 0;
    CHECK(sce_forge_cursor_read_vle_u16(&u16, &value16) == SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW, "65536 is past a u16");
    static const uint8_t past32[] = {0x80, 0x80, 0x80, 0x80, 0x10};
    sce_forge_cursor_t u32 = sce_forge_cursor_init(past32, sizeof(past32));
    uint32_t value32 = 0;
    CHECK(sce_forge_cursor_read_vle_u32(&u32, &value32) == SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW, "2^32 is past a u32");
    /* A ninth byte with its high bit set is data, not a flag asking for a tenth. */
    static const uint8_t high[] = {0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01};
    sce_forge_cursor_t high32 = sce_forge_cursor_init(high, sizeof(high));
    CHECK(sce_forge_cursor_read_vle_u32(&high32, &value32) == SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW, "2^63 is past a u32");
    sce_forge_cursor_t high64 = sce_forge_cursor_init(high, sizeof(high));
    uint64_t value64 = 0;
    CHECK(sce_forge_cursor_read_vle_u64(&high64, &value64) == SCE_FORGE_CODEC_OK && value64 == ((uint64_t)1 << 63),
          "a ninth byte's high bit is bit 63");
    CHECK(sce_forge_cursor_remaining(&high64) == 1, "the read stops after the ninth byte");
}

static void a_chain_the_buffer_ends_inside_needs_more_bytes(void) {
    static const uint8_t short_chain[] = {0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80};
    sce_forge_cursor_t u16 = sce_forge_cursor_init(short_chain, sizeof(short_chain));
    uint16_t value16 = 0;
    CHECK(sce_forge_cursor_read_vle_u16(&u16, &value16) == SCE_FORGE_CODEC_NEED_MORE_BYTES,
          "a u16 chain cut short needs more bytes");
    sce_forge_cursor_t u64 = sce_forge_cursor_init(short_chain, sizeof(short_chain));
    uint64_t value64 = 0;
    CHECK(sce_forge_cursor_read_vle_u64(&u64, &value64) == SCE_FORGE_CODEC_NEED_MORE_BYTES,
          "a u64 chain cut short needs more bytes");
}

int main(void) {
    a_u16_reads_each_encoding_of_a_value_it_holds_alike();
    a_u32_and_a_u64_read_a_padded_encoding_of_a_value_they_hold();
    a_narrow_read_refuses_the_first_value_past_its_width();
    a_chain_the_buffer_ends_inside_needs_more_bytes();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    puts("vle_runtime_test: all checks passed");
    return 0;
}
