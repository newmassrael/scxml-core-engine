// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test172 — C11 AOT runner.
//
// W3C SCXML 6.2: when `<send>` carries `eventexpr`, the processor MUST
// evaluate the expression at send time and use the result as the event
// name. test172 declares `Var1 = 'event1'` then reassigns it to
// `'event2'` in onentry before the send, so the eventexpr must yield
// "event2" — not the initial "event1". The first transition matches
// event2 → pass; the wildcard catch-all event="*" → fail must NOT fire.

#include <stdio.h>

#include "test172_sm.h"

int main(void) {
    test172_t sm;
    test172_init(&sm);
    test172_run(&sm);

    int rc = test172_in_state(&sm, TEST172_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test172: FAIL — active = 0x%08x\n", (unsigned)test172_active_states(&sm));
    }
    test172_destroy(&sm);
    return rc;
}
