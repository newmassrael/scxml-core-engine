// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#include <stdio.h>

#include "test421_sm.h"

int main(void) {
    test421_t sm;
    test421_init(&sm);
    test421_run(&sm);

    int rc = test421_in_state(&sm, TEST421_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test421: FAIL — active = 0x%08x\n", (unsigned)test421_active_states(&sm));
    }
    test421_destroy(&sm);
    return rc;
}
