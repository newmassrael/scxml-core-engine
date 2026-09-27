// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches
// the parent, however many it sends in one tick — C11 AOT.
//
// This backend bounds every event queue at SCE_MAX_EVENTS; the target is
// built with it raised above the fixture's 110 events, so what is measured
// is the delivery, not the default bound.
//
// Fixture:
// integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_child_may_send_many_events_in_one_tick ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_child_may_send_many_events_in_one_tick_sm.h"

typedef a_child_may_send_many_events_in_one_tick_t sm_t;

int main(void) {
    sm_t sm;
    a_child_may_send_many_events_in_one_tick_init(&sm);
    a_child_may_send_many_events_in_one_tick_run(&sm);

    a_child_may_send_many_events_in_one_tick_event_with_meta_t carrier = {0};
    carrier.event = A_CHILD_MAY_SEND_MANY_EVENTS_IN_ONE_TICK_EVENT_FINISH;
    a_child_may_send_many_events_in_one_tick_raise_external(&sm, &carrier);
    a_child_may_send_many_events_in_one_tick_run(&sm);

    int ok = 1;
    int64_t ticks = -1;
    if (!a_child_may_send_many_events_in_one_tick_ended_in(&sm, A_CHILD_MAY_SEND_MANY_EVENTS_IN_ONE_TICK_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }
    if (!a_child_may_send_many_events_in_one_tick_ticks(&sm, &ticks) || ticks != 110) {
        fprintf(stderr, "FAIL: ticks = %lld, want 110: every event the child sent arrives before `finish`\n",
                (long long)ticks);
        ok = 0;
    }
    a_child_may_send_many_events_in_one_tick_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: all 110 events the child sent in one tick arrived\n");
    return 0;
}
