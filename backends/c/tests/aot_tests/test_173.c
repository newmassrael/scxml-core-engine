// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test173 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send targetexpr="...">` evaluates the target string at
// send time, not at parse time. The fixture initialises Var1 to an
// invalid session id, then reassigns Var1 = '#_internal' in onentry just
// before the send. The runtime targetexpr eval reads the current Var1
// ('#_internal'), routes the event to the internal queue, and the
// receiving transition matches event1 → pass. If the eval used the initial
// value the dispatch would raise error.execution and the wildcard would
// route to fail.

#include <stdio.h>

#include "test173_sm.h"

int main(void) {
    test173_t sm;
    test173_init(&sm);
    test173_run(&sm);

    int rc = test173_in_state(&sm, TEST173_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test173: FAIL — active = 0x%08x\n", (unsigned)test173_active_states(&sm));
    }
    test173_destroy(&sm);
    return rc;
}
