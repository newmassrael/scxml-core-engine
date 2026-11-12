// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test421 — C11 AOT runner.
//
// W3C SCXML 3.12.1 + App.D.2: internal events take priority over
// external ones, and the processor keeps draining the internal queue
// past dequeues that match no enabled transition until it finds one
// that does. The fixture's s1 onentry sends externalEvent (external)
// then raises internalEvent1..4 (internal). With s11 active the
// internal pops of internalEvent1/2 fall through (no matching
// transition), internalEvent3 fires the s11→s12 transition, then
// internalEvent4 fires s12→pass. The `s1 transition event=externalEvent
// target=fail` would steal pass if external were ever drained before
// the full internal sequence completes.
//
// W3C SCXML 3.12.1 + App.D.2: internal-priority drain past non-matching
// dequeues — internalEvent1/2 fall through (no matching transition with s11
// active), internalEvent3 fires s11→s12, internalEvent4 fires s12→pass; the
// s1-level <transition event=externalEvent target=fail> never runs because
// the final state is reached before external dequeue

#include <stdio.h>

#include "../require_started.h"
#include "test421_sm.h"

int main(void) {
    test421_t sm;
    REQUIRE_STARTED(test421_init(&sm), TEST421_START_OK);
    test421_run(&sm);

    int rc = test421_ended_in(&sm, TEST421_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test421: FAIL — active = 0x%08x\n", (unsigned)test421_active_states(&sm));
    }
    test421_destroy(&sm);
    return rc;
}
