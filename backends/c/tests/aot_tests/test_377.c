// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test377 — C11 AOT runner.
//
// W3C SCXML 3.13: When a state declares multiple <onexit> blocks they
// must execute in document order. test377's s0 has block 1 raising
// event1 and block 2 raising event2, with an eventless transition to
// s1. After the eventless transition fires, the FIFO internal queue
// must observe [event1, event2] so s1's `event="event1"` arm fires
// (routing to s2) and s2's `event="event2"` arm completes the run at
// `pass`. Any reordering would land on `event="*" target="fail"`.
//
// Fixture-only landing: the existing C11 execute_exit_actions walks
// state.on_exit_blocks (a list-of-lists, one inner list per onexit
// element) in document order and emits each block's actions inline,
// matching the spec semantics. needs_script_engine=false so the MCU
// zero-deps profile is preserved — this fixture links without lua54.

#include <stdio.h>

#include "test377_sm.h"

int main(void) {
    test377_t sm;
    test377_init(&sm);
    test377_run(&sm);

    int rc = test377_in_state(&sm, TEST377_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test377: FAIL — active = 0x%08x\n", (unsigned)test377_active_states(&sm));
    }
    test377_destroy(&sm);
    return rc;
}
