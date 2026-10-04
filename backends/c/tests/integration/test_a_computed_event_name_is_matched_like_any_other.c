// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// C11 AOT, on a manual clock so the two delayed sends fall due when the test says.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_computed_event_name_is_matched_like_any_other ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "a_computed_event_name_is_matched_like_any_other_sm.h"

int main(void) {
    a_computed_event_name_is_matched_like_any_other_t sm;
    a_computed_event_name_is_matched_like_any_other_init_with_clock(&sm, sce_clock_manual(0u));

    // Two sends wait 20 ms each; 10 ms steps give the run all the time it can use
    // and no more.
    for (int elapsed = 0; elapsed < 1000 && !a_computed_event_name_is_matched_like_any_other_is_in_final_state(&sm);
         elapsed += 10) {
        a_computed_event_name_is_matched_like_any_other_advance_time_ms(&sm, 10u);
    }

    const int reached = a_computed_event_name_is_matched_like_any_other_ended_in(
        &sm, A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_PASS);
    if (!reached) {
        (void)fprintf(stderr, "a_computed_event_name_is_matched_like_any_other: FAIL - a computed name was dropped "
                              "(`request.new`, `other.thing`, `request.again`, `last.one`), or matched but told "
                              "shorter than the name it was sent under. W3C 3.12.1 matches it by token and 5.10 "
                              "makes `_event.name` the whole name, so the event's `name` must ride beside its "
                              "member.\n");
    }
    a_computed_event_name_is_matched_like_any_other_destroy(&sm);
    return reached ? 0 : 1;
}
