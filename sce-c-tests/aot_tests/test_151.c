// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test151 — C11 AOT runner.
//
// W3C SCXML 4.6 sibling of test150: this fixture's pass-guard reads the
// `index` variable (`typeof Var5 !== 'undefined'`) instead of the `item`
// variable. Because the index lane in `lua_foreach_no_body` declares
// `{{ index }} = nil` and reassigns it on every iteration (via `_i - 1`),
// test151 trips on the same code path that test150 does — but with the
// declaration/reassignment of the *index* slot as the load-bearing edge.
// No extra codegen is needed beyond what test150 already activated; this
// runner just adds a second consumer for the same path.

#include <stdio.h>

#include "test151_sm.h"

int main(void) {
    test151_t sm;
    test151_init(&sm);
    test151_run(&sm);

    test151_state_t final = test151_get_current_state(&sm);
    int rc = (final == TEST151_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test151: FAIL — final state = %d\n", (int)final);
    }
    test151_destroy(&sm);
    return rc;
}
