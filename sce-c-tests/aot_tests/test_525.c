// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test525 — C11 AOT runner.
//
// W3C SCXML 4.6: `<foreach>` operates on a shallow copy — modifying `Var2` (the iterated array) inside the body does not change the iteration order or count, so the body fires exactly Var2.length times against the captured snapshot. Existing lua_foreach_body_{prologue,iter,epilogue} macros (76133c71) drive the snapshot semantics.

#include <stdio.h>

#include "test525_sm.h"

int main(void) {
    test525_t sm;
    test525_init(&sm);
    test525_run(&sm);

    int rc = test525_in_state(&sm, TEST525_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test525: FAIL — active = 0x%08x\n", (unsigned)test525_active_states(&sm));
    }
    test525_destroy(&sm);
    return rc;
}
