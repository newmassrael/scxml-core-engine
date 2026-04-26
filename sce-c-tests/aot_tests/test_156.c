// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test156 — C11 AOT runner.
//
// W3C SCXML 4.6: An error inside the body of a <foreach> must cease
// execution of the loop. test156 iterates Var3 = [1,2,3], where each
// iteration first runs `<assign location="Var1" expr="Var1 + 1"/>` and
// then `<assign location="Var5" expr="undefined.invalidProperty"/>`.
// The second assign transpiles to `Var5 = (nil.invalidProperty)`, which
// is an attempt-to-index-nil error in Lua — luaL_dostring returns
// non-zero. The first iteration must increment Var1 once and the loop
// must break before the second iteration starts; the transition guard
// `cond="Var1 == 1"` then routes to pass.
//
// First consumer of the in-foreach assign error path
// (`lua_assign_in_foreach_body` macro): the chunk's return value is
// checked, the error message is popped off the Lua stack, and the
// enclosing C `for` loop terminates via `break`.

#include <stdio.h>

#include "test156_sm.h"

int main(void) {
    test156_t sm;
    test156_init(&sm);
    test156_run(&sm);

    test156_state_t final = test156_get_current_state(&sm);
    int rc = (final == TEST156_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test156: FAIL — final state = %d\n", (int)final);
    }
    test156_destroy(&sm);
    return rc;
}
