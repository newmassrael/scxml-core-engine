// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test403b — C11 AOT runner.
//
// Adversarial fixture for remove_conflicting_transitions: two active
// region leaves (p0s1, p0s2) under <parallel id="p0"> both bubble into
// the same parent transition `<transition event="event1">`, which gets
// pushed onto the enabled list twice. Conflict resolution must dedup
// via exit-set intersection so the body (`Var1 = Var1 + 1`) fires
// exactly once — pass cond is `Var1 == 1`, fire-twice would route to
// fail. Pins the W3C App.D.2 "optimal enabled set is a set" semantics
// that handle_microstep relies on (multi-leaf bubble dedup).

#include <stdio.h>

#include "test403b_sm.h"

int main(void) {
    test403b_t sm;
    test403b_init(&sm);
    test403b_run(&sm);

    int rc = test403b_in_state(&sm, TEST403B_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test403b: FAIL — active = 0x%08x\n", (unsigned)test403b_active_states(&sm));
    }
    test403b_destroy(&sm);
    return rc;
}
