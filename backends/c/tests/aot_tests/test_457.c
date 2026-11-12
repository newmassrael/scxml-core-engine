// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test457 — C11 AOT runner.
//
// W3C SCXML 4.6 + 5.4: `<foreach array=...>` requires the value to satisfy
// `instanceof Array`. s0's array=Var4 (number 7) fails the new `_isArray`
// runtime check (mirrors cpp `LuaEngine::_isArray` lua_pushcfunction at
// LuaEngine.cpp:463-483, T3-inlined as a Lua-script via dostring) → `_sce_n <
// 0` arm raises error.execution + `return;` (W3C 5.10
// stop-subsequent-content; axis I block helper terminates only that block).
// s0→s1: array=Var5=[1,2,3] passes `_isArray` but item="'continue'" emits an
// illegal Lua identifier — `lua_foreach_body_iter` chunk fails on first
// iteration, raises error.execution and `break`s the C loop (body actions
// never executed → Var1 stays 0). s1→s2 verifies Var1==0 (executable content
// never ran), then s3 iterates [1,2,3] cleanly summing to Var6=6 → cond
// Var6==6 → pass.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST457_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test457_sm.h"

int main(void) {
    test457_t sm;
    REQUIRE_STARTED(test457_init(&sm), TEST457_START_OK);
    test457_run(&sm);

    int rc = test457_ended_in(&sm, TEST457_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test457: FAIL — active = 0x%08x\n", (unsigned)test457_active_states(&sm));
    }
    test457_destroy(&sm);
    return rc;
}
