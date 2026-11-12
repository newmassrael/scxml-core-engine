// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test378 — C11 AOT runner.
//
// W3C SCXML 3.13: same separate-block semantics for `<onexit>` — block 1's
// invalid-target `<send>` aborts after raising error.execution, block 2's
// `<assign Var1=Var1+1>` still runs on its own helper, then s0→s1 transition
// fires and s1's cond `Var1==2` routes to pass. Verifies the per-block helper
// split applies symmetrically to `execute_exit_actions`.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST378_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test378_sm.h"

int main(void) {
    test378_t sm;
    REQUIRE_STARTED(test378_init(&sm), TEST378_START_OK);
    test378_run(&sm);

    int rc = test378_ended_in(&sm, TEST378_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test378: FAIL — active = 0x%08x\n", (unsigned)test378_active_states(&sm));
    }
    test378_destroy(&sm);
    return rc;
}
