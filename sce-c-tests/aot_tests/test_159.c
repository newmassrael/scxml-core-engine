// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test159 — C11 AOT runner.
//
// W3C SCXML 6.2 + 5.10: when an executable element raises an error,
// the SCXML processor MUST skip every subsequent element in the same
// onentry/onexit/transition block. test159's s0 onentry sends to an
// invalid target ("!invalid") which raises error.execution; the
// following `<assign Var1 = Var1 + 1>` must therefore not run. The
// first transition `cond="Var1 == 1" target="fail"` then evaluates
// false (Var1 stayed 0), and the unconditional fallback transition
// routes to pass. Any leak that lets the assign run flips the cond
// true and the runner ends in fail.

#include <stdio.h>

#include "test159_sm.h"

int main(void) {
    test159_t sm;
    test159_init(&sm);
    test159_run(&sm);

    int rc = test159_in_state(&sm, TEST159_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test159: FAIL — active = 0x%08x\n", (unsigned)test159_active_states(&sm));
    }
    test159_destroy(&sm);
    return rc;
}
