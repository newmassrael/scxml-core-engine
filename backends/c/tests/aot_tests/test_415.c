// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test415 — C11 AOT runner.
//
// W3C SCXML 3.7: A top-level <final> state halts the state machine
// immediately. test415's initial state IS the final state ("final"), so
// the engine must enter `final`, recognize it as terminal, and exit
// without draining the queue. The fixture's onentry on `final` raises
// `event1`; the test verifies the machine halts at `final` regardless
// of that pending event (the spec is "halt", not "halt and process
// pending events").
//
// Custom verdict (the only fixture in the C11 harness without a `pass`
// state): treat the run as PASS if the engine ends in TEST415_STATE_FINAL.
// The fixture comment notes this is a manual test ("there is no
// platform-independent way to test that event1 is not processed");
// reaching the final state at all is the strongest automatable check.

#include <stdio.h>

#include "test415_sm.h"

int main(void) {
    test415_t sm;
    test415_init(&sm);
    test415_run(&sm);

    int rc = test415_in_state(&sm, TEST415_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test415: FAIL — active = 0x%08x\n", (unsigned)test415_active_states(&sm));
    }
    test415_destroy(&sm);
    return rc;
}
