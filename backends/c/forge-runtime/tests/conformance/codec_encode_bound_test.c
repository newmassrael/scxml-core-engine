/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * A generated codec holds a byte string, a text, a list or a TLV chain in an array
 * of its declared bound beside a length the caller sets. The encode of a length
 * above that bound is refused with SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE before the
 * first byte is written: it neither leaves the payload out of a frame that
 * reports success nor reads past the array. A length at the bound is written whole.
 * Each case takes one shape of the generated encode: a positional tail and
 * length-ref, a streaming text (its length held by a sibling), a gated byte string
 * (held in the optional value of the bytes), a repeat and a TLV chain.
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "codec_length_ref.h"
#include "codec_present_if_tail.h"
#include "codec_repeat_basic.h"
#include "codec_tail.h"
#include "codec_tlv_chain_basic.h"
#include "codec_zenoh_locator.h"

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

#define CANARY 0xA5

/* A buffer the encode has not written to is every byte the canary. */
static bool untouched(const uint8_t *buf, size_t n) {
    for (size_t i = 0; i < n; ++i) {
        if (buf[i] != CANARY) {
            return false;
        }
    }
    return true;
}

/* A tail of exactly its bound is written whole; one byte above is refused, with no
 * byte written and the writer where it began. */
static void a_tail_above_its_bound_is_refused(void) {
    static codec_tail_t v;
    uint8_t buf[CODEC_TAIL_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    v.msg_id = 7;
    v.status = 1;
    memset(v.payload, 0x42, sizeof v.payload);
    v.payload_len = sizeof v.payload;
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_tail_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK, "a tail at its bound encodes");
    CHECK(len == CODEC_TAIL_MAX_BYTES, "a tail at its bound is written whole");
    CHECK(buf[0] == 7 && buf[1] == 1 && buf[2] == 0x42 && buf[len - 1] == 0x42, "the bytes of a tail at its bound");

    v.payload_len = sizeof v.payload + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_tail_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a tail above its bound is refused");
    CHECK(len == 0, "a refused tail leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused tail writes no byte, not even the fields before it");
}

/* The same for a payload whose length a sibling field carries. */
static void a_length_ref_payload_above_its_bound_is_refused(void) {
    static codec_length_ref_t v;
    uint8_t buf[CODEC_LENGTH_REF_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    v.msg_id = 3;
    v.len = (uint8_t)sizeof v.payload;
    memset(v.payload, 0x24, sizeof v.payload);
    v.payload_len = sizeof v.payload;
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_length_ref_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "a payload at its bound encodes");
    CHECK(len == CODEC_LENGTH_REF_MAX_BYTES, "a payload at its bound is written whole");

    v.payload_len = sizeof v.payload + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_length_ref_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a payload above its bound is refused");
    CHECK(len == 0, "a refused payload leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused payload writes no byte");
}

/* A text whose length is its sibling's VLE number: the bound is the text's own
 * array, not the width of the number that counts it. */
static void a_text_above_its_bound_is_refused(void) {
    static codec_zenoh_locator_t v;
    uint8_t buf[CODEC_ZENOH_LOCATOR_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    memset(v.locator, 'a', sizeof v.locator);
    v.locator_len = sizeof v.locator;
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_zenoh_locator_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "a text at its bound encodes");
    CHECK(len == 2 + sizeof v.locator, "a text at its bound is its two-byte length and its bytes");

    v.locator_len = sizeof v.locator + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_zenoh_locator_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a text above its bound is refused");
    CHECK(len == 0, "a refused text leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused text writes no byte, not even its length");
}

/* A gated byte string is held in the optional value of the bytes: the length it
 * holds is refused where the value is present, and an absent value has none. */
static void a_gated_byte_string_above_its_bound_is_refused(void) {
    static codec_present_if_tail_t v;
    uint8_t buf[CODEC_PRESENT_IF_TAIL_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    v.flags = 1;
    v.payload.present = true;
    memset(v.payload.value.data, 0x5A, sizeof v.payload.value.data);
    v.payload.value.len = sizeof v.payload.value.data;
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_present_if_tail_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "a gated byte string at its bound encodes");
    CHECK(len == CODEC_PRESENT_IF_TAIL_MAX_BYTES, "a gated byte string at its bound is written whole");

    v.payload.value.len = sizeof v.payload.value.data + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_present_if_tail_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a gated byte string above its bound is refused");
    CHECK(len == 0, "a refused gated byte string leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused gated byte string writes no byte, not even the flags");

    v.flags = 0;
    v.payload.present = false;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_present_if_tail_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "an absent byte string has no length to refuse");
    CHECK(len == 1 && buf[0] == 0, "an absent byte string writes the flags alone");
}

/* A list of more entries than its max-count: refused before the count byte. */
static void a_repeat_above_its_bound_is_refused(void) {
    static codec_repeat_basic_t v;
    uint8_t buf[CODEC_REPEAT_BASIC_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    v.num_frags = (uint8_t)(sizeof v.frags / sizeof v.frags[0]);
    v.frags_len = sizeof v.frags / sizeof v.frags[0];
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_repeat_basic_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "a list at its bound encodes");
    CHECK(len == CODEC_REPEAT_BASIC_MAX_BYTES, "a list at its bound is written whole");

    v.frags_len = sizeof v.frags / sizeof v.frags[0] + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_repeat_basic_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a list above its bound is refused");
    CHECK(len == 0, "a refused list leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused list writes no byte, not even its count");
}

/* A TLV chain of more entries than its max-depth. */
static void a_tlv_chain_above_its_bound_is_refused(void) {
    static codec_tlv_chain_basic_t v;
    uint8_t buf[CODEC_TLV_CHAIN_BASIC_MAX_BYTES + 8];
    size_t len = 99;

    memset(&v, 0, sizeof v);
    v.extensions_len = sizeof v.extensions / sizeof v.extensions[0];
    memset(buf, CANARY, sizeof buf);
    CHECK(codec_tlv_chain_basic_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK,
          "a chain at its bound encodes");
    CHECK(len == 1 + 2 * (sizeof v.extensions / sizeof v.extensions[0]), "a chain at its bound is written whole");

    v.extensions_len = sizeof v.extensions / sizeof v.extensions[0] + 1;
    memset(buf, CANARY, sizeof buf);
    len = 99;
    CHECK(codec_tlv_chain_basic_encode_to_buf(&v, buf, sizeof buf, &len) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a chain above its bound is refused");
    CHECK(len == 0, "a refused chain leaves the writer where it began");
    CHECK(untouched(buf, sizeof buf), "a refused chain writes no byte, not even its header");
}

int main(void) {
    a_tail_above_its_bound_is_refused();
    a_length_ref_payload_above_its_bound_is_refused();
    a_text_above_its_bound_is_refused();
    a_gated_byte_string_above_its_bound_is_refused();
    a_repeat_above_its_bound_is_refused();
    a_tlv_chain_above_its_bound_is_refused();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
