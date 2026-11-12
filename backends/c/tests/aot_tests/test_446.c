// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test446 — C11 AOT runner.
//
// W3C SCXML 5.3.1 + 5.3.2 + B.2: inline `<data id="var1">[1, 2, 3]</data>`
// already lowered by `to_lua_expr` (array-literal pass) to a Lua table —
// `var1 = ({1, 2, 3})` evaluates fine. The new `var.src` branch reads
// `file:test446.txt` (identical content `[1, 2, 3]`) at codegen time via the
// `read_data_src` filter and threads the content through the same emit shape
// as inline content, so `var2 = ({1,2,3})` also lands as a Lua table. The
// cond `var instanceof Array` lowers to `_isArray(var)`
// (transform_instanceof_patterns at lua_transformer.rs:605) — both vars now
// satisfy it and the s0→s1→pass chain matches.

#include <stdio.h>

#include "../require_started.h"
#include "test446_sm.h"

int main(void) {
    test446_t sm;
    REQUIRE_STARTED(test446_init(&sm), TEST446_START_OK);
    test446_run(&sm);

    int rc = test446_ended_in(&sm, TEST446_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test446: FAIL — active = 0x%08x\n", (unsigned)test446_active_states(&sm));
    }
    test446_destroy(&sm);
    return rc;
}
