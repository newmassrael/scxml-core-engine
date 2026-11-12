// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test552 — C11 AOT runner.
//
// W3C SCXML 5.3.1: `<data id="Var1" src="file:test552.txt"/>` reads the
// sidecar (literal `2`) at codegen time; `to_lua_expr` returns `2` unchanged
// (numeric literals pass through), `Var1 = (2)` evaluates as the Lua number
// 2, and the cond `typeof Var1 !== 'undefined'` lowers to `Var1 ~= nil`
// (transform_typeof_undefined) → truthy → s0→pass.

#include <stdio.h>

#include "../require_started.h"
#include "test552_sm.h"

int main(void) {
    test552_t sm;
    REQUIRE_STARTED(test552_init(&sm), TEST552_START_OK);
    test552_run(&sm);

    int rc = test552_ended_in(&sm, TEST552_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test552: FAIL — active = 0x%08x\n", (unsigned)test552_active_states(&sm));
    }
    test552_destroy(&sm);
    return rc;
}
