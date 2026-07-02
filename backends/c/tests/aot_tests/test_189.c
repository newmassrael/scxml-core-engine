// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test189 — C11 AOT runner.
//
// W3C SCXML C.1: <send target="#_internal"> routes to the internal queue
// (W3C 3.12.1 high-priority FIFO), while <send> with no target defaults
// to the external (low-priority) queue. test189's s0 onentry first sends
// event2 with no target (external) then sends event1 with
// target="#_internal" (internal). App.D.2's mainEventLoop drains the
// internal queue before the external one, so event1 must match its
// transition to `pass` before event2 ever reaches its transition to
// `fail`. Any leak that pushes the #_internal send onto the external
// queue inverts the dispatch order and the runner ends in fail.

#include <stdio.h>

#include "test189_sm.h"

int main(void) {
    test189_t sm;
    test189_init(&sm);
    test189_run(&sm);

    int rc = test189_in_state(&sm, TEST189_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test189: FAIL — active = 0x%08x\n", (unsigned)test189_active_states(&sm));
    }
    test189_destroy(&sm);
    return rc;
}
