/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * A value a generated codec holds that may be absent is held in the struct
 * `sce/forge/optional.h` declares for its element (docs/adr/0055, section 8): the
 * struct a statechart's variable of the same type is held in, so a value read
 * from a codec is taken whole by one. A variable of the runtime's own type
 * receives the member by assignment, which compiles only where the two are one
 * type, and the codec is handed the runtime's own constructors for what it
 * writes. The bytes and the refusals of the codecs are the conformance harness's
 * (codec_cbor_map, codec_content_line_event), which this file does not repeat.
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "codec_cbor_map.h"
#include "codec_content_line_event.h"

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

/* A map whose optional entries are one present and one absent: decoded, the
 * present one is a value of `sce_optional_uint64_t` and the absent one a value
 * of `sce_optional_bool_t`, each taken whole by a variable of that type. */
static void a_cbor_entry_that_may_be_absent_is_taken_whole(void) {
    static codec_cbor_map_t sent;
    static codec_cbor_map_t got;
    uint8_t buf[CODEC_CBOR_MAP_MAX_BYTES];
    size_t len = 0;
    sce_forge_cursor_t cursor;

    memset(&sent, 0, sizeof sent);
    memcpy(sent.source, "ab", 2);
    sent.source_len = 2;
    memset(sent.id, 0x11, sizeof sent.id);
    sent.id_len = sizeof sent.id;
    sent.pattern = 3;
    sent.payload[0] = 9;
    sent.payload_len = 1;
    sent.deadline = sce_optional_uint64_some(UINT64_C(1700000000000));
    sent.urgent = sce_optional_bool_none();

    CHECK(codec_cbor_map_encode_to_buf(&sent, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK, "encode");
    cursor = sce_forge_cursor_init(buf, len);
    CHECK(codec_cbor_map_decode(&cursor, &got) == SCE_FORGE_CODEC_OK, "decode");

    {
        sce_optional_uint64_t deadline = got.deadline;
        sce_optional_bool_t urgent = got.urgent;
        CHECK(deadline.present && deadline.value == UINT64_C(1700000000000), "the present entry is its value");
        CHECK(sce_optional_uint64_eq(deadline, sce_optional_uint64_some(UINT64_C(1700000000000))),
              "the present entry is equal to the runtime's value of it");
        CHECK(!urgent.present, "the absent entry is not present");
        CHECK(sce_optional_bool_eq(urgent, sce_optional_bool_none()), "the absent entry is the runtime's absent value");
    }
}

/* A component with a property, a parameter, a number, an integer and a bool that
 * may be absent: decoded, a text is a `sce_optional_string_<N>_t` of its bound
 * and the others are the runtime's own, each taken whole by a variable of its
 * type, and what is absent stays absent. */
static void a_content_line_entry_that_may_be_absent_is_taken_whole(void) {
    static codec_content_line_event_t sent;
    static codec_content_line_event_t got;
    uint8_t buf[CODEC_CONTENT_LINE_EVENT_MAX_BYTES];
    size_t len = 0;
    sce_forge_cursor_t cursor;

    memset(&sent, 0, sizeof sent);
    memcpy(sent.uid, "u1", 2);
    sent.uid_len = 2;
    memcpy(sent.dtstart, "20260101", 8);
    sent.dtstart_len = 8;
    memcpy(sent.dtstart_tzid, "UTC", 3);
    sent.dtstart_tzid_len = 3;
    sent.summary = sce_optional_string_24_some(sce_static_string_24_of("meet"));
    sent.organizer = sce_optional_string_40_some(sce_static_string_40_of("o1"));
    sent.organizer_cn = sce_optional_string_24_some(sce_static_string_24_of("Ann"));
    sent.sequence = sce_optional_uint16_some(7u);
    sent.priority = sce_optional_int32_some(-3);
    sent.all_day = sce_optional_bool_none();

    CHECK(codec_content_line_event_encode_to_buf(&sent, buf, sizeof buf, &len) == SCE_FORGE_CODEC_OK, "encode");
    cursor = sce_forge_cursor_init(buf, len);
    CHECK(codec_content_line_event_decode(&cursor, &got) == SCE_FORGE_CODEC_OK, "decode");

    {
        sce_optional_string_24_t summary = got.summary;
        sce_optional_string_40_t organizer = got.organizer;
        sce_optional_string_24_t organizer_cn = got.organizer_cn;
        sce_optional_uint16_t sequence = got.sequence;
        sce_optional_int32_t priority = got.priority;
        sce_optional_bool_t all_day = got.all_day;
        sce_optional_string_160_t description = got.description;
        CHECK(summary.present && strcmp(summary.value.data, "meet") == 0, "a text property is its text");
        CHECK(sce_optional_string_24_eq(summary, sce_optional_string_24_some(sce_static_string_24_of("meet"))),
              "a text is equal to the runtime's value of it");
        CHECK(organizer.present && strcmp(organizer.value.data, "o1") == 0, "a property that has a parameter");
        CHECK(organizer_cn.present && strcmp(organizer_cn.value.data, "Ann") == 0, "its parameter");
        CHECK(sequence.present && sequence.value == 7u, "an unsigned integer");
        CHECK(priority.present && priority.value == -3, "a signed integer");
        CHECK(!all_day.present, "a bool that is absent");
        CHECK(!description.present, "a text that is absent");
    }
}

int main(void) {
    a_cbor_entry_that_may_be_absent_is_taken_whole();
    a_content_line_entry_that_may_be_absent_is_taken_whole();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
