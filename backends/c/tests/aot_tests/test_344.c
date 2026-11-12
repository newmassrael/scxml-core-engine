// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test344 — C11 AOT runner.
//
// W3C SCXML 5.9 (test344): a transition cond that cannot be evaluated as a
// boolean MUST place `error.execution` on the internal queue.
// `conf:nonBoolean=""` lowers to `cond="return"` (TXMLConverter.cpp:387),
// which after `to_lua_guard` becomes `_scxml_truthy(return)` — `return` is a
// Lua keyword so luaL_dostring fails. The new `lua_eval_guard` else-branch
// (mirrors cpp `LuaEngine::evaluateBoolean` chunk-failure path) pops the
// error message, raises `error.execution`, and leaves `_trans_pass=false` so
// the surrounding bare fall-through transition still fires. test309 (already
// in the C11 list) verified that the false-fall + eventless next-transition
// path stays observable: the queued error.execution sits behind the eventless
// fire, and pass is reached before the queue drain.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST344_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test344_sm.h"

int main(void) {
    test344_t sm;
    REQUIRE_STARTED(test344_init(&sm), TEST344_START_OK);
    test344_run(&sm);

    int rc = test344_ended_in(&sm, TEST344_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test344: FAIL — active = 0x%08x\n", (unsigned)test344_active_states(&sm));
    }
    test344_destroy(&sm);
    return rc;
}
