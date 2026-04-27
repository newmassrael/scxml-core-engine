// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test153 — C11 AOT runner.
//
// W3C SCXML 4.6: <foreach> visits array elements in document order. The
// fixture compares each Var2 against the running max in Var1; if any
// Var2 is not strictly greater than its predecessor the body's <else>
// branch zeros Var4 to flag the failure. With Var3 = [1,2,3] the if-true
// branch must fire on every iteration and Var4 stays at 1.
//
// This is the second consumer of the body-bearing foreach path
// (introduced for test155) and the first consumer that nests <if>/<else>
// inside <foreach> — exercising the recursive emit_action call from the
// foreach body into the if-branch macro.

#include <stdio.h>

#include "test153_sm.h"

int main(void) {
    test153_t sm;
    test153_init(&sm);
    test153_run(&sm);

    int rc = test153_in_state(&sm, TEST153_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test153: FAIL — active = 0x%08x\n", (unsigned)test153_active_states(&sm));
    }
    test153_destroy(&sm);
    return rc;
}
