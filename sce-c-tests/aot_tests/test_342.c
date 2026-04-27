// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test342 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send eventexpr="Var1"/>` evaluates Var1 at send time
// (currently 'foo') and uses the result as the dispatched event's name.
// The receiving transition then captures `_event.name` into Var2; the
// final cond `Var1 === Var2` (strict equality, transpiled to Lua `==`)
// confirms the dispatched event's name field matches the original
// expression value. Pinned the eventexpr branch (test172) and the
// _event.name binding (test318) being co-active in one microstep.

#include <stdio.h>

#include "test342_sm.h"

int main(void) {
    test342_t sm;
    test342_init(&sm);
    test342_run(&sm);

    int rc = test342_in_state(&sm, TEST342_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test342: FAIL — active = 0x%08x\n", (unsigned)test342_active_states(&sm));
    }
    test342_destroy(&sm);
    return rc;
}
