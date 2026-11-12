// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C11 machine reads a composed payload at any depth (docs/adr/0086, section 6),
// run with no script engine, as the Rust, Kotlin, Go, Python and C++ machines do
// (`backends/python/tests/integration/static_datamodel/test_a_composed_payload_is_read_at_any_depth.py`).
//
// A delivery whose every list is full is lifted into the machine's own types, and
// each read at depths 1 to 3 is what a host reads back. A delivery one past a bound
// at depth 2 is refused whole (section 4.2): none of the content runs, and
// `error.execution` is raised once. A value a host injects through the seam is
// written as its JSON part by part, each part held to its type as it is written
// (section 5.4): within its bounds it is read as any delivery, and a list past the
// room of its type or a text with no end in its buffer makes the seam answer false
// and raise nothing. A list taken whole into a variable is the machine's own copy.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml,
// beside the schemas it imports (event_batch.scxml, event_batch_item.scxml and
// enum_signal_level.scxml), generated at build time by
// `sce_generate_static_integration_c_test(static_composed_payload ...)` in
// `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "static_composed_payload_sm.h"

typedef static_composed_payload_t sm_t;

static int failures = 0;

static void expect_u32(const char *what, uint32_t got, uint32_t want, const char *why) {
    if (got != want) {
        fprintf(stderr, "FAIL: %s = %u, want %u: %s\n", what, (unsigned)got, (unsigned)want, why);
        failures++;
    }
}

/* One item of the batch: `tags` holds `tags` texts. */
static int item_json(char *out, size_t cap, unsigned id, unsigned tags) {
    static const char *const words[] = {"\"tag0\"", "\"tag1\"", "\"tag2\"", "\"tag3\"", "\"tag4\""};
    char list[64] = "";
    for (unsigned n = 0u; n < tags; ++n) {
        if (n > 0u) {
            strcat(list, ",");
        }
        strcat(list, words[n]);
    }
    return snprintf(out, cap,
                    "{\"id\":%u,\"amount\":\"9007199254740993\",\"label\":\"abcdefgh\",\"level\":\"high\","
                    "\"due\":null,\"blob\":\"wxyz\",\"tags\":[%s]}",
                    id, list);
}

/* A delivery of `batch.sent` whose every list is full, the second item's `tags`
   holding `second_tags` texts. */
static void batch_json(char *out, size_t cap, unsigned second_tags) {
    char first[192];
    char second[192];
    char third[192];
    (void)item_json(first, sizeof(first), 1u, 4u);
    (void)item_json(second, sizeof(second), 2u, second_tags);
    (void)item_json(third, sizeof(third), 3u, 4u);
    (void)snprintf(out, cap,
                   "{\"items\":[%s,%s,%s],\"grid\":[[1,2],[3,4]],\"levels\":[\"low\",\"critical\"],"
                   "\"maybe\":[7,null],\"spare\":[1,2],\"note\":\"abcd\",\"late\":[5,6]}",
                   first, second, third);
}

/* The machine, started: static storage, since its queues hold a slot of
   `_event.data` an entry. */
static sm_t machine;

static sm_t *started(void) {
    REQUIRE_STARTED(static_composed_payload_init(&machine), STATIC_COMPOSED_PAYLOAD_START_OK);
    static_composed_payload_run(&machine);
    return &machine;
}

/* A machine that took one delivery of `batch.sent` carrying `data`. */
static sm_t *delivered(const char *data) {
    sm_t *sm = started();
    static_composed_payload_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = STATIC_COMPOSED_PAYLOAD_EVENT_BATCH_SENT;
    evt.data = data;
    static_composed_payload_raise_external(sm, &evt);
    static_composed_payload_run(sm);
    return sm;
}

static void a_full_delivery_is_read_at_every_depth(void) {
    char data[1024];
    batch_json(data, sizeof(data), 4u);
    sm_t *sm = delivered(data);
    expect_u32("refusals", static_composed_payload_get_refusals(sm), 0u, "a delivery of its type is no error");
    // Depth 1: the length of a list of the payload.
    expect_u32("item_count", static_composed_payload_get_item_count(sm), 3u, "three items");
    // Depth 2: a loop over each item's list, and a field of an element read through its index.
    expect_u32("tag_count", static_composed_payload_get_tag_count(sm), 12u, "four tags an item");
    expect_u32("word_count", static_composed_payload_get_word_count(sm), 12u, "each tag walked");
    expect_u32("id_sum", static_composed_payload_get_id_sum(sm), 6u, "ids 1, 2 and 3");
    // A list of lists, walked at both depths and read by two indices.
    expect_u32("cell_sum", static_composed_payload_get_cell_sum(sm), 10u, "cells 1 to 4");
    expect_u32("cell", static_composed_payload_get_cell(sm), 3u, "grid[1][0]");
    // Depth 3: the length of the first item's `tags`.
    expect_u32("first_tags", static_composed_payload_get_first_tags(sm), 4u, "items[0].tags");
    // An element that may be absent, present.
    const sce_optional_uint16_t first_maybe = static_composed_payload_get_first_maybe(sm);
    if (!first_maybe.present || first_maybe.value != 7u) {
        fprintf(stderr, "FAIL: first_maybe is not 7\n");
        failures++;
    }
    // A list taken whole into a list of a larger bound.
    const sce_forge_uint8_view_t late = static_composed_payload_get_late(sm);
    if (late.len != 2u || late.data[0] != 5u || late.data[1] != 6u) {
        fprintf(stderr, "FAIL: late is not [5, 6]\n");
        failures++;
    }
    static_composed_payload_destroy(sm);
}

static void a_list_past_its_bound_at_depth_two_is_refused_whole(void) {
    char data[1024];
    batch_json(data, sizeof(data), 5u);
    sm_t *sm = delivered(data);
    expect_u32("refusals", static_composed_payload_get_refusals(sm), 1u, "error.execution reached the document once");
    expect_u32("item_count", static_composed_payload_get_item_count(sm), 0u, "none of the content ran");
    expect_u32("late", (uint32_t)static_composed_payload_get_late(sm).len, 0u, "nothing of the delivery was kept");
    static_composed_payload_destroy(sm);
}

/* A payload a host builds: one item whose `tags` hold two texts, a list of one
   list, the first level, a present and an absent value, no spare, and `late`. */
static void host_payload(static_composed_payload_batch_sent_payload_t *payload) {
    memset(payload, 0, sizeof(*payload));
    static_composed_payload_record_item_t *item = &payload->items.data[0];
    payload->items.len = 1u;
    item->id = 4u;
    item->amount = 2;
    memcpy(item->label.data, "abc", 4u);
    item->level = ENUM_SIGNAL_LEVEL_LOW;
    item->due.present = false;
    memcpy(item->blob.data, "wx", 2u);
    item->blob.len = 2u;
    item->tags.len = 2u;
    memcpy(item->tags.data[0].data, "t0", 3u);
    memcpy(item->tags.data[1].data, "t1", 3u);
    payload->grid.len = 1u;
    payload->grid.data[0].len = 2u;
    payload->grid.data[0].data[0] = 1u;
    payload->grid.data[0].data[1] = 2u;
    payload->levels.len = 1u;
    payload->levels.data[0] = ENUM_SIGNAL_LEVEL_LOW;
    payload->maybe.len = 2u;
    payload->maybe.data[0].present = true;
    payload->maybe.data[0].value = 9u;
    payload->maybe.data[1].present = false;
    payload->spare.present = false;
    memcpy(payload->note.data, "ab", 3u);
    payload->late.len = 2u;
    payload->late.data[0] = 5u;
    payload->late.data[1] = 6u;
}

static void a_value_a_host_injects_within_its_bounds_is_read_as_any_delivery(void) {
    static static_composed_payload_batch_sent_payload_t payload;
    host_payload(&payload);
    sm_t *sm = started();
    if (!static_composed_payload_raise_batch_sent_typed(sm, &payload)) {
        fprintf(stderr, "FAIL: a payload within every bound was refused by the seam\n");
        failures++;
    }
    static_composed_payload_run(sm);
    expect_u32("refusals", static_composed_payload_get_refusals(sm), 0u, "the seam's JSON fits its schema");
    expect_u32("item_count", static_composed_payload_get_item_count(sm), 1u, "one item");
    expect_u32("tag_count", static_composed_payload_get_tag_count(sm), 2u, "two tags");
    expect_u32("id_sum", static_composed_payload_get_id_sum(sm), 4u, "the item's id");
    expect_u32("cell_sum", static_composed_payload_get_cell_sum(sm), 3u, "cells 1 and 2");
    const sce_optional_uint16_t first_maybe = static_composed_payload_get_first_maybe(sm);
    if (!first_maybe.present || first_maybe.value != 9u) {
        fprintf(stderr, "FAIL: first_maybe is not 9\n");
        failures++;
    }
    const sce_forge_uint8_view_t late = static_composed_payload_get_late(sm);
    if (late.len != 2u || late.data[0] != 5u || late.data[1] != 6u) {
        fprintf(stderr, "FAIL: late is not [5, 6]\n");
        failures++;
    }
    // The variable holds a copy of its own: what the host writes after does not reach it.
    payload.late.data[0] = 99u;
    if (static_composed_payload_get_late(sm).data[0] != 5u) {
        fprintf(stderr, "FAIL: the variable shares the list the host handed in\n");
        failures++;
    }
    static_composed_payload_destroy(sm);
}

static void a_value_a_host_injects_past_its_type_is_refused_where_it_is_written(void) {
    static static_composed_payload_batch_sent_payload_t payload;
    // `late` is a list<uint8>[2]: three elements are no value of its type.
    host_payload(&payload);
    payload.late.len = 3u;
    sm_t *sm = started();
    if (static_composed_payload_raise_batch_sent_typed(sm, &payload)) {
        fprintf(stderr, "FAIL: a list past the room of its type was sent\n");
        failures++;
    }
    static_composed_payload_run(sm);
    expect_u32("refusals", static_composed_payload_get_refusals(sm), 0u, "nothing was sent, so nothing was refused");
    expect_u32("item_count", static_composed_payload_get_item_count(sm), 0u, "nothing was raised");
    static_composed_payload_destroy(sm);
    // A text with no end inside its buffer, at depth 2.
    host_payload(&payload);
    memset(payload.items.data[0].label.data, 'x', sizeof(payload.items.data[0].label.data));
    sm = started();
    if (static_composed_payload_raise_batch_sent_typed(sm, &payload)) {
        fprintf(stderr, "FAIL: a text with no end in its buffer was sent\n");
        failures++;
    }
    static_composed_payload_run(sm);
    expect_u32("item_count", static_composed_payload_get_item_count(sm), 0u, "nothing was raised");
    static_composed_payload_destroy(sm);
}

int main(void) {
    a_full_delivery_is_read_at_every_depth();
    a_list_past_its_bound_at_depth_two_is_refused_whole();
    a_value_a_host_injects_within_its_bounds_is_read_as_any_delivery();
    a_value_a_host_injects_past_its_type_is_refused_where_it_is_written();
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("PASS: a composed payload is read at any depth, and a host's value is held to its type\n");
    return 0;
}
