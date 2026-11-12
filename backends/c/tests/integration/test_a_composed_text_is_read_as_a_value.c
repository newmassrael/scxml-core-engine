// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C11 machine reads, as values, the texts and the byte strings a composed payload
// holds, at depths 1 to 3, as it reads a variable of the same type (docs/adr/0086,
// section 6), run with no script engine. C11 holds each such text in the buffer of
// its bound, and an expression reads it as the buffer's text: a guard compares the
// payload's text, and the content it guards measures it, compares a text field of an
// element read through its index, and walks the items, comparing a field of the
// loop's record item, an element of a list of texts read through its index and the
// item of a loop over that list, taking a text into a text variable, and measuring
// and comparing a byte string. A delivery whose text the guard does not take runs the
// other transition, which measures the text and reads no further.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_composed_text.scxml,
// beside the schemas it imports (event_batch.scxml, event_batch_item.scxml and
// enum_signal_level.scxml), generated at build time by
// `sce_generate_static_integration_c_test(static_composed_text ...)` in
// `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "static_composed_text_sm.h"

typedef static_composed_text_t sm_t;

static int failures = 0;

static void expect_u32(const char *what, uint32_t got, uint32_t want, const char *why) {
    if (got != want) {
        fprintf(stderr, "FAIL: %s = %u, want %u: %s\n", what, (unsigned)got, (unsigned)want, why);
        failures++;
    }
}

static void expect_bool(const char *what, bool got, bool want, const char *why) {
    if (got != want) {
        fprintf(stderr, "FAIL: %s = %s, want %s: %s\n", what, got ? "true" : "false", want ? "true" : "false", why);
        failures++;
    }
}

static void expect_text(const char *what, const char *got, const char *want, const char *why) {
    if (got == NULL || strcmp(got, want) != 0) {
        fprintf(stderr, "FAIL: %s = \"%s\", want \"%s\": %s\n", what, got == NULL ? "(null)" : got, want, why);
        failures++;
    }
}

/* A delivery of `batch.sent` whose `note` is `note`: three items, the first
   labelled "ab" with the byte string "ab" and the tags "ab" and "x", the second
   labelled "cd" with "wxyz" and the tags "x", "ab" and "ab", the third labelled
   "ef" with no bytes and no tags. */
static void batch_json(char *out, size_t cap, const char *note) {
    (void)snprintf(out, cap,
                   "{\"items\":["
                   "{\"id\":1,\"amount\":\"9007199254740993\",\"label\":\"ab\",\"level\":\"high\","
                   "\"due\":null,\"blob\":\"ab\",\"tags\":[\"ab\",\"x\"]},"
                   "{\"id\":2,\"amount\":\"9007199254740993\",\"label\":\"cd\",\"level\":\"low\","
                   "\"due\":null,\"blob\":\"wxyz\",\"tags\":[\"x\",\"ab\",\"ab\"]},"
                   "{\"id\":3,\"amount\":\"9007199254740993\",\"label\":\"ef\",\"level\":\"critical\","
                   "\"due\":null,\"blob\":\"\",\"tags\":[]}"
                   "],\"grid\":[[1,2],[3,4]],\"levels\":[\"low\"],\"maybe\":[],\"spare\":null,"
                   "\"note\":\"%s\",\"late\":[5]}",
                   note);
}

/* The machine, started: static storage, since its queues hold a slot of
   `_event.data` an entry. */
static sm_t machine;

/* A machine that took one delivery of `batch.sent` carrying `data`. */
static sm_t *delivered(const char *data) {
    REQUIRE_STARTED(static_composed_text_init(&machine), STATIC_COMPOSED_TEXT_START_OK);
    static_composed_text_run(&machine);
    static_composed_text_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = STATIC_COMPOSED_TEXT_EVENT_BATCH_SENT;
    evt.data = data;
    static_composed_text_raise_external(&machine, &evt);
    static_composed_text_run(&machine);
    return &machine;
}

static void each_text_and_byte_string_is_read_as_a_value(void) {
    char data[1024];
    batch_json(data, sizeof(data), "ab");
    sm_t *sm = delivered(data);
    expect_u32("refusals", static_composed_text_get_refusals(sm), 0u, "a delivery of its type is no error");
    // Depth 1: the guard compares the payload's text, and the content measures it.
    expect_bool("note_is_ab", static_composed_text_get_note_is_ab(sm), true, "the guard took \"ab\"");
    expect_u32("note_len", static_composed_text_get_note_len(sm), 2u, "\"ab\" is two bytes");
    // Depth 3, through a checked index: the first item's label.
    expect_bool("first_label_is_ab", static_composed_text_get_first_label_is_ab(sm), true, "items[0].label");
    // A field of the loop's record item, an element of its list of texts read through
    // its index, and the item of a loop over that list.
    expect_u32("labels_ab", static_composed_text_get_labels_ab(sm), 1u, "only the first label is \"ab\"");
    expect_u32("first_tags_ab", static_composed_text_get_first_tags_ab(sm), 1u, "only the first item's tags[0]");
    expect_u32("words_ab", static_composed_text_get_words_ab(sm), 3u, "one tag of the first, two of the second");
    // A text taken into a text variable: the last item's label.
    expect_text("last_label", static_composed_text_get_last_label(sm), "ef", "the third item's label");
    // A byte string measured and compared.
    expect_u32("blob_bytes", static_composed_text_get_blob_bytes(sm), 6u, "two, four and no bytes");
    expect_u32("blobs_ab", static_composed_text_get_blobs_ab(sm), 1u, "only the first item's bytes are \"ab\"");
    static_composed_text_destroy(sm);
}

static void a_text_the_guard_does_not_take_runs_the_other_transition(void) {
    char data[1024];
    batch_json(data, sizeof(data), "abcd");
    sm_t *sm = delivered(data);
    expect_u32("refusals", static_composed_text_get_refusals(sm), 0u, "a delivery of its type is no error");
    expect_bool("note_is_ab", static_composed_text_get_note_is_ab(sm), false, "\"abcd\" is not \"ab\"");
    expect_u32("note_len", static_composed_text_get_note_len(sm), 4u, "\"abcd\" is four bytes");
    expect_u32("labels_ab", static_composed_text_get_labels_ab(sm), 0u, "the guarded content did not run");
    expect_u32("words_ab", static_composed_text_get_words_ab(sm), 0u, "the guarded content did not run");
    expect_text("last_label", static_composed_text_get_last_label(sm), "", "the guarded content did not run");
    static_composed_text_destroy(sm);
}

int main(void) {
    each_text_and_byte_string_is_read_as_a_value();
    a_text_the_guard_does_not_take_runs_the_other_transition();
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("PASS: a text and a byte string a composed payload holds are read as values\n");
    return 0;
}
