// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//
// W3C SCXML 5.8: top-level `<script>Var1 = 1</script>` evaluates at document
// load time before any state entry — the new global_scripts loop in
// lua_init_engine routes the body through `to_lua_script` + `luaL_dostring`,
// binding Var1 in the lua datamodel before s0 entry; the s0 cond `Var1 == 1`
// then matches and routes to pass. Mirrors cpp
// `scriptengine_helpers.jinja2:165-179` (model.global_scripts loop emit
// shape).

#include <stdio.h>

#include "../require_started.h"
#include "test302_sm.h"

int main(void) {
    test302_t sm;
    REQUIRE_STARTED(test302_init(&sm), TEST302_START_OK);
    test302_run(&sm);

    int rc = test302_ended_in(&sm, TEST302_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test302: FAIL — active = 0x%08x\n", (unsigned)test302_active_states(&sm));
    }
    test302_destroy(&sm);
    return rc;
}
