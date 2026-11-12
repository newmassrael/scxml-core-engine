// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//
// W3C SCXML 6.2 + 5.10: <send target="!invalid"> raises error.execution and
// stops subsequent executable content — the assign immediately after the
// invalid-target send must be skipped so Var1 stays 0, the cond `Var1==1`
// transition then misses, and the unconditional fall-through routes to pass

#include <stdio.h>

#include "../require_started.h"
#include "test159_sm.h"

int main(void) {
    test159_t sm;
    REQUIRE_STARTED(test159_init(&sm), TEST159_START_OK);
    test159_run(&sm);

    int rc = test159_ended_in(&sm, TEST159_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test159: FAIL — active = 0x%08x\n", (unsigned)test159_active_states(&sm));
    }
    test159_destroy(&sm);
    return rc;
}
