// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0086, section 8.6 (c): the data of an event a host hands a machine
// by name, which the machine's document declares no schema for, is bounded by
// what the host ABI declares for it, SCE_UNTYPED_EVENT_DATA_MAX_LEN. A machine
// with no script engine holds that much in its slot of `_event.data`, and
// refuses whole, counted and named, data past the slot.
//
// The host hands `note` by name twice: first with data of exactly the ABI's
// bound, which the machine keeps and takes; then with data one byte past the
// slot, which the machine refuses, so it stays where the first `note` left it
// and its count of refused data names `note`.
//
// Fixture: integration_resources/untyped_data_by_name/untyped_data_by_name.scxml
// (C-only).
//
// Regeneration: automatic at build time via
// `sce_generate_static_integration_c_test(untyped_data_by_name ...)` in
// `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "untyped_data_by_name_sm.h"

// The data a host may count on handing any machine, and the data one byte
// past this machine's slot, whose bytes hold its text and the NUL.
static char at_the_bound[SCE_UNTYPED_EVENT_DATA_MAX_LEN + 1u];
static char past_the_slot[UNTYPED_DATA_BY_NAME_DATA_LEN + 1u];

// Hand the machine `note` by name, as a host does, with `data`, and run it to
// quiescence.
static void hand_note(untyped_data_by_name_t *sm, const char *data) {
    sce_forwarded_event_t arriving;
    memset(&arriving, 0, sizeof(arriving));
    sce_copy_bounded_id(arriving.name, "note");
    arriving.data = data;
    untyped_data_by_name_raise_external_forwarded(sm, &arriving);
    untyped_data_by_name_run(sm);
}

int main(void) {
    memset(at_the_bound, 'x', sizeof(at_the_bound) - 1u);
    at_the_bound[sizeof(at_the_bound) - 1u] = '\0';
    memset(past_the_slot, 'y', sizeof(past_the_slot) - 1u);
    past_the_slot[sizeof(past_the_slot) - 1u] = '\0';

    untyped_data_by_name_t sm;
    REQUIRE_STARTED(untyped_data_by_name_init(&sm), UNTYPED_DATA_BY_NAME_START_OK);
    untyped_data_by_name_run(&sm);
    int bad = 0;

    hand_note(&sm, at_the_bound);
    if (!untyped_data_by_name_in_state(&sm, UNTYPED_DATA_BY_NAME_STATE_HEARD) ||
        untyped_data_by_name_refused_event_data(&sm) != 0u) {
        (void)fprintf(stderr,
                      "untyped_data_by_name: FAIL - data of the %u bytes the host ABI declares was not "
                      "kept: %u refused\n",
                      (unsigned)SCE_UNTYPED_EVENT_DATA_MAX_LEN, (unsigned)untyped_data_by_name_refused_event_data(&sm));
        bad = 1;
    }

    hand_note(&sm, past_the_slot);
    untyped_data_by_name_event_t refused = UNTYPED_DATA_BY_NAME_EVENT_NOTE;
    const bool named = untyped_data_by_name_last_refused_event_data(&sm, &refused);
    if (!untyped_data_by_name_in_state(&sm, UNTYPED_DATA_BY_NAME_STATE_HEARD) ||
        untyped_data_by_name_refused_event_data(&sm) != 1u || !named || refused != UNTYPED_DATA_BY_NAME_EVENT_NOTE) {
        (void)fprintf(stderr,
                      "untyped_data_by_name: FAIL - data past the slot was not refused whole and "
                      "named: %u refused, %s\n",
                      (unsigned)untyped_data_by_name_refused_event_data(&sm), named ? "named" : "not named");
        bad = 1;
    }

    untyped_data_by_name_destroy(&sm);
    return bad;
}
