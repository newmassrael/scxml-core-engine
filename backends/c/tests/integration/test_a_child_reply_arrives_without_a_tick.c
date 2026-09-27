// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: a reply an invoked child has already sent is on the
// parent's external queue, so a host that only hands the machine events
// still sees it ahead of its own later events — C11 AOT.
//
// Fixture:
// integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_child_reply_arrives_without_a_tick ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_child_reply_arrives_without_a_tick_sm.h"

typedef a_child_reply_arrives_without_a_tick_t sm_t;

int main(void) {
    sm_t sm;
    a_child_reply_arrives_without_a_tick_init(&sm);
    a_child_reply_arrives_without_a_tick_run(&sm);

    a_child_reply_arrives_without_a_tick_event_with_meta_t carrier = {0};
    carrier.event = A_CHILD_REPLY_ARRIVES_WITHOUT_A_TICK_EVENT_FINISH;
    a_child_reply_arrives_without_a_tick_raise_external(&sm, &carrier);
    a_child_reply_arrives_without_a_tick_run(&sm);

    int ok = 1;
    int64_t hellos = -1;
    if (!a_child_reply_arrives_without_a_tick_ended_in(&sm, A_CHILD_REPLY_ARRIVES_WITHOUT_A_TICK_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }
    if (!a_child_reply_arrives_without_a_tick_hellos(&sm, &hellos) || hellos != 1) {
        fprintf(stderr, "FAIL: hellos = %lld, want 1: the child's start-time reply arrives before `finish`\n",
                (long long)hellos);
        ok = 0;
    }
    a_child_reply_arrives_without_a_tick_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: the child's start-time reply arrived before the host's next event\n");
    return 0;
}
