// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#include <stdio.h>

#include "test304_sm.h"

int main(void) {
    test304_t sm;
    test304_init(&sm);
    test304_run(&sm);

    int rc = test304_in_state(&sm, TEST304_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test304: FAIL — active = 0x%08x\n", (unsigned)test304_active_states(&sm));
    }
    test304_destroy(&sm);
    return rc;
}
