// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test460 — C11 AOT runner.
//
// W3C SCXML 4.6 + B.2 shallow copy: `<foreach array="Var1">` body executes
// `Var1 = [].concat(Var1, [4])` per iteration — the new `_concat` Lua helper
// (mirrors cpp `LuaEngine::_concat` at LuaEngine.cpp:519-539, T3-inlined as a
// Lua-script via dostring) creates a fresh table, leaving the foreach
// prologue's `_sce_foreach_arr` referencing the original [1,2,3] table by
// Lua's table-by-reference semantics. The iteration count therefore stays at
// 3 — Var2 reaches 3 → cond Var2==3 → pass; verifies the shallow-copy
// invariant emerges naturally from Lua reference binding without any explicit
// table-copy in the prologue.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST460_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test460_sm.h"

int main(void) {
    test460_t sm;
    REQUIRE_STARTED(test460_init(&sm), TEST460_START_OK);
    test460_run(&sm);

    int rc = test460_ended_in(&sm, TEST460_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test460: FAIL — active = 0x%08x\n", (unsigned)test460_active_states(&sm));
    }
    test460_destroy(&sm);
    return rc;
}
