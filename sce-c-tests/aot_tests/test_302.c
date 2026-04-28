// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test302 — C11 AOT runner.
//
// W3C SCXML 5.8: a top-level `<script>Var1 = 1</script>` must be
// evaluated at document load time, before any state is entered.
// The fixture has no `<datamodel>`; the script alone declares Var1
// and binds it to the integer 1. test302's s0 then has a single
// guarded transition `cond="Var1 == 1" target="pass"` and an
// unconditional fall-through to fail.
//
// C11 lifts the script body via the new global_scripts loop in
// `scriptengine.jinja2::lua_init_engine` — `to_lua_script` runs the
// ECMAScript→Lua transformer over the body and emits `luaL_dostring`
// before any state machine entry. If the chunk fails (e.g. syntax
// error), error.execution is queued but the SM still runs; for the
// happy path (this fixture) the binding lands cleanly and the
// guard's `Var1 == 1` reads the bound value.

#include <stdio.h>

#include "test302_sm.h"

int main(void) {
    test302_t sm;
    test302_init(&sm);
    test302_run(&sm);

    int rc = test302_in_state(&sm, TEST302_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test302: FAIL — active = 0x%08x\n",
                      (unsigned)test302_active_states(&sm));
    }
    test302_destroy(&sm);
    return rc;
}
