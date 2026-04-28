// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test303 — C11 AOT runner.
//
// W3C SCXML 5.8: `<script>` is executable content like `<assign>` or
// `<raise>` — it runs in document order with the surrounding actions.
// test303's s0 onentry fires `<assign Var1=2>` then `<script>Var1 = 1</script>`
// in that order, so after the block Var1 is the script's later
// assignment (1), not the earlier `<assign>` value (2). The guarded
// transition `cond="Var1 == 1"` then matches.
//
// C11 lifts the inline-script branch in `state_machine.c.jinja2::emit_action`
// — `to_lua_script` over the body, then `luaL_dostring` inside the
// per-block helper. The block ordering is the same as the cpp
// reference (parse_executable_content preserves doc-order in
// `state.on_entry_blocks`).

#include <stdio.h>

#include "test303_sm.h"

int main(void) {
    test303_t sm;
    test303_init(&sm);
    test303_run(&sm);

    int rc = test303_in_state(&sm, TEST303_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test303: FAIL — active = 0x%08x\n",
                      (unsigned)test303_active_states(&sm));
    }
    test303_destroy(&sm);
    return rc;
}
