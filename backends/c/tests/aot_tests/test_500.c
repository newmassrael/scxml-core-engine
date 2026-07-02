// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test500 — C11 AOT runner.
//
// W3C SCXML 5.10 + C.1: `_ioprocessors['scxml']['location']` must be
// bound (non-nil) at startup so the datamodel `<data id=Var1
// expr="_ioprocessors['scxml']['location']"/>` resolves to a defined
// value. cpp `LuaEngine::setupSystemVariables` populates the table with
// `_ioprocessors[proc] = {location = proc}` for every processor in the
// session's I/O list (cpp `StateMachine.cpp:3107` seeds it as
// `{"scxml"}`); the C11 emit lifts the same shape into a single
// `luaL_dostring` at lua_init_engine. The cond `typeof Var1 !==
// 'undefined'` (lua-transpiled to `Var1 ~= nil`) flips truthy on the
// bound value and routes s0→pass.

#include <stdio.h>

#include "test500_sm.h"

int main(void) {
    test500_t sm;
    test500_init(&sm);
    test500_run(&sm);

    int rc = test500_in_state(&sm, TEST500_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test500: FAIL — active = 0x%08x\n", (unsigned)test500_active_states(&sm));
    }
    test500_destroy(&sm);
    return rc;
}
