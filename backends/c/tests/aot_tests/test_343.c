// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test343 — C11 AOT runner.
//
// W3C SCXML 5.7: <param location="foo"/> referencing an undeclared variable
// triggers ECMAScript ReferenceError semantics — `_scxml_declared` set +
// `rawget(_G, base)` mirror cpp `LuaEngine::isUndeclaredSimpleVariable` so
// the param raises error.execution and continues with empty data; s1 then
// receives `done.state.s0` and routes to pass
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST343_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test343_sm.h"

int main(void) {
    test343_t sm;
    REQUIRE_STARTED(test343_init(&sm), TEST343_START_OK);
    test343_run(&sm);

    int rc = test343_ended_in(&sm, TEST343_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test343: FAIL — active = 0x%08x\n", (unsigned)test343_active_states(&sm));
    }
    test343_destroy(&sm);
    return rc;
}
