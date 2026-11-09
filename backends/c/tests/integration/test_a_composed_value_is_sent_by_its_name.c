// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A C11 machine sends a composed value by its name (docs/adr/0086, section 5.2),
// run with no script engine, as the Rust, Kotlin, Go, Python and C++ machines do
// (`backends/python/tests/integration/static_datamodel/test_a_composed_value_is_sent_by_its_name.py`).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four lists
// and an optional list named as the parts of the payload they are, and a list
// variable the machine took the payload's `late` into, each written where its pair
// stands. The delivery that comes back is lifted as any delivery is (section 4), so
// it is refused whole if anything written is outside its type; it is not, and every
// value read from it at depths 1 to 3 is what the first delivery held.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml,
// beside the schemas it imports, generated at build time by
// `sce_generate_static_integration_c_test(static_composed_send ...)` in
// `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_composed_send_sm.h"

typedef static_composed_send_t sm_t;

static int failures = 0;

static void expect_u32(const char *what, uint32_t got, uint32_t want, const char *why) {
    if (got != want) {
        fprintf(stderr, "FAIL: %s = %u, want %u: %s\n", what, (unsigned)got, (unsigned)want, why);
        failures++;
    }
}

/* A delivery of `batch.sent` whose every list is full but the second item's
   `tags`, an element of `maybe` absent and `spare` present. */
static const char BATCH[] =
    "{\"items\":["
    "{\"id\":1,\"amount\":\"9007199254740993\",\"label\":\"abcdefgh\",\"level\":\"high\",\"due\":null,"
    "\"blob\":\"wxyz\",\"tags\":[\"tag0\",\"tag1\",\"tag2\",\"tag3\"]},"
    "{\"id\":2,\"amount\":\"9007199254740993\",\"label\":\"abcdefgh\",\"level\":\"high\",\"due\":null,"
    "\"blob\":\"wxyz\",\"tags\":[\"tag0\",\"tag1\",\"tag2\"]},"
    "{\"id\":3,\"amount\":\"9007199254740993\",\"label\":\"abcdefgh\",\"level\":\"high\",\"due\":null,"
    "\"blob\":\"wxyz\",\"tags\":[\"tag0\",\"tag1\",\"tag2\",\"tag3\"]}],"
    "\"grid\":[[1,2],[3,4]],\"levels\":[\"low\",\"critical\"],\"maybe\":[7,null],\"spare\":[1,2],"
    "\"note\":\"abcd\",\"late\":[5,6]}";

/* The machine: static storage, since its queues hold a slot of `_event.data` an
   entry. */
static sm_t machine;

int main(void) {
    sm_t *sm = &machine;
    static_composed_send_init(sm);
    static_composed_send_run(sm);
    static_composed_send_event_with_meta_t evt;
    memset(&evt, 0, sizeof(evt));
    evt.event = STATIC_COMPOSED_SEND_EVENT_BATCH_SENT;
    evt.data = BATCH;
    static_composed_send_raise_external(sm, &evt);
    static_composed_send_run(sm);

    expect_u32("refusals", static_composed_send_get_refusals(sm), 0u,
               "the delivery the machine sent itself fits its schema");
    expect_u32("deliveries", static_composed_send_get_deliveries(sm), 2u,
               "the forwarded delivery came back on the internal queue");
    // Depth 1: the lengths of a list of records and a list of enum values.
    expect_u32("item_count", static_composed_send_get_item_count(sm), 3u, "three items");
    expect_u32("level_count", static_composed_send_get_level_count(sm), 2u, "two levels");
    // Depth 2: each item's list, and a field of an element read by its index.
    expect_u32("tag_count", static_composed_send_get_tag_count(sm), 11u, "four, three and four tags");
    expect_u32("id_sum", static_composed_send_get_id_sum(sm), 6u, "ids 1, 2 and 3");
    // A list of lists, walked at both depths.
    expect_u32("cell_sum", static_composed_send_get_cell_sum(sm), 10u, "cells 1 to 4");
    // An element that may be absent, present.
    const sce_optional_uint16_t first_maybe = static_composed_send_get_first_maybe(sm);
    if (!first_maybe.present || first_maybe.value != 7u) {
        fprintf(stderr, "FAIL: first_maybe is not 7\n");
        failures++;
    }
    // The list variable, sent by its name.
    expect_u32("late_sum", static_composed_send_get_late_sum(sm), 11u, "5 and 6");
    const sce_forge_uint8_view_t late = static_composed_send_get_late(sm);
    if (late.len != 2u || late.data[0] != 5u || late.data[1] != 6u) {
        fprintf(stderr, "FAIL: late is not [5, 6]\n");
        failures++;
    }
    static_composed_send_destroy(sm);
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    printf("PASS: a composed value sent by its name comes back whole\n");
    return 0;
}
