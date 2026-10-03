// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — C11 AOT
// local-invoke path.
//
// A transition on `request` takes `request.new` by whole-token matching, and
// `_event.name` is then the name the event was sent under, not the descriptor it
// was matched through. The public IRP suite never reads a name the document does
// not write, so a machine that is told the shorter one passes all of it.
//
// Fixture:
// integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_event_keeps_the_name_it_was_sent_under ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "an_event_keeps_the_name_it_was_sent_under_sm.h"

int main(void) {
    an_event_keeps_the_name_it_was_sent_under_t sm;
    an_event_keeps_the_name_it_was_sent_under_init(&sm);

    // No `<send delay>` in this fixture: the child tells the parent it is `ready`
    // during its own `_init`, the parent answers `request.new`, and the child's
    // verdict rides home as `arrivedAsSent` or `arrivedShortened`.
    an_event_keeps_the_name_it_was_sent_under_run(&sm);

    int rc =
        an_event_keeps_the_name_it_was_sent_under_ended_in(&sm, AN_EVENT_KEEPS_THE_NAME_IT_WAS_SENT_UNDER_STATE_PASS)
            ? 0
            : 1;
    if (rc != 0) {
        fprintf(stderr,
                "an_event_keeps_the_name_it_was_sent_under: FAIL — the child "
                "reported `arrivedShortened` (or never heard `request.new`): "
                "`request.new` took the transition on `request` but "
                "`_event.name` told the child `request`. W3C 5.10 makes the "
                "name the one the event was sent under; the arrival name must "
                "ride in `event_with_meta_t.name`. Diagnostic: in_PASS=%d "
                "in_FAIL=%d in_phase=%d\n",
                an_event_keeps_the_name_it_was_sent_under_ended_in(
                    &sm, AN_EVENT_KEEPS_THE_NAME_IT_WAS_SENT_UNDER_STATE_PASS),
                an_event_keeps_the_name_it_was_sent_under_ended_in(
                    &sm, AN_EVENT_KEEPS_THE_NAME_IT_WAS_SENT_UNDER_STATE_FAIL),
                an_event_keeps_the_name_it_was_sent_under_in_state(
                    &sm, AN_EVENT_KEEPS_THE_NAME_IT_WAS_SENT_UNDER_STATE_PHASE));
    }
    an_event_keeps_the_name_it_was_sent_under_destroy(&sm);
    return rc;
}
