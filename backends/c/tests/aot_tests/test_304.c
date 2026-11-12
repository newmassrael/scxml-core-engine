// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test304 — C11 AOT runner.
//
// W3C SCXML 5.8: a variable declared by a top-level `<script>` is
// accessible from the datamodel like any other `<data>` entry.
// test304's body is `<script>Var1 = 1</script>` (same as test302) —
// a Var1 binding becomes available before any state is entered, and
// s0's `cond="Var1 == 1"` transition matches it. The differential
// against test302 is observability: test304 confirms cond evaluation
// reads the script-bound Var1 (not just that load-time eval ran).
//
// W3C SCXML 5.8: a variable declared by a top-level `<script>` is reachable
// from the datamodel without a `<data>` declaration — same fixture shape as
// test302 over a different observable claim (cond reads the script-bound
// Var1, not just that load-time eval ran). The C11 emit path is identical
// (global_scripts loop in lua_init_engine), and lua's global-by-default scope
// means `Var1 = 1` in the script automatically lands as a datamodel-visible
// global.

#include <stdio.h>

#include "../require_started.h"
#include "test304_sm.h"

int main(void) {
    test304_t sm;
    REQUIRE_STARTED(test304_init(&sm), TEST304_START_OK);
    test304_run(&sm);

    int rc = test304_ended_in(&sm, TEST304_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test304: FAIL — active = 0x%08x\n", (unsigned)test304_active_states(&sm));
    }
    test304_destroy(&sm);
    return rc;
}
