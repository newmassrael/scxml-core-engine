// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test444 — C11 AOT runner.
//
// W3C SCXML 5.2: `<data id="var1" expr="1"/>` creates an ECMAScript
// variable accessible from a transition cond. The cond `++var1==2` is
// transpiled by `lua_transformer::transform_increment_decrement` into
// `(function() var1 = var1 + 1 return var1 end)() == 2`, so the
// pre-increment side-effect lands on the datamodel and the comparison
// fires against the post-increment value. PASS branch is taken when
// the cond evaluates to true, exercising the closure-based ++ shim.

#include <stdio.h>

#include "test444_sm.h"

int main(void) {
    test444_t sm;
    test444_init(&sm);
    test444_run(&sm);

    int rc = test444_in_state(&sm, TEST444_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test444: FAIL — active = 0x%08x\n", (unsigned)test444_active_states(&sm));
    }
    test444_destroy(&sm);
    return rc;
}
