// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test496 — C11 AOT runner.
//
// W3C SCXML 6.2: when `<send>` cannot dispatch because the resolved
// target is not a reachable URI, the platform raises
// `error.communication` on the internal queue. The fixture's s0
// onentry sends `event` with the SCXMLEventProcessor type literal and
// `targetexpr="undefined"` (lua transpiles the ECMAScript identifier
// to nil; tostring(nil) yields the literal `"nil"`), then raises
// `foo` for the wildcard fall-through. The targetexpr arm
// (commit `1a7e92da`) classifies the buffer: not empty, not
// `#_internal`, not `'!'`-prefixed → final clause raises
// `error.communication`. App.D.2's internal-priority drain dispatches
// the error before the queued `foo`, matching the s0→pass transition
// before the wildcard could fire.

#include <stdio.h>

#include "test496_sm.h"

int main(void) {
    test496_t sm;
    test496_init(&sm);
    test496_run(&sm);

    int rc = test496_in_state(&sm, TEST496_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test496: FAIL — active = 0x%08x\n", (unsigned)test496_active_states(&sm));
    }
    test496_destroy(&sm);
    return rc;
}
