// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test155 — C11 AOT runner.
//
// W3C SCXML 4.6: <foreach> executes its body once per element. test155
// sums Var3 = [1,2,3] into Var1 (initially 0) by `<assign location="Var1"
// expr="Var1 + Var2"/>` inside the loop. The transition `cond="Var1 ==
// 6"` then verifies the body ran exactly three times with Var2 walking
// through 1, 2, 3.
//
// This is the first consumer of the body-bearing foreach path
// (lua_foreach_body_prologue / _iter / _epilogue): the C-side `for` loop
// drives the iteration count, and each iteration invokes a per-step
// luaL_dostring to bind the item variable before the assign action runs.

#include <stdio.h>

#include "test155_sm.h"

int main(void) {
    test155_t sm;
    test155_init(&sm);
    test155_run(&sm);

    int rc = test155_in_state(&sm, TEST155_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test155: FAIL — active = 0x%08x\n", (unsigned)test155_active_states(&sm));
    }
    test155_destroy(&sm);
    return rc;
}
