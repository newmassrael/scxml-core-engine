// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test417 — C11 AOT runner.
//
// W3C SCXML 3.4 + 3.7.1: when every region of a <parallel> has an active
// direct <final> child, raise done.state.<parallel> — the s1p1 entry walks
// regions s1p11/s1p12, eventless transitions move s1p111→s1p11final and
// s1p121→s1p12final, and the second compound-final entry triggers the
// codegen-time `_region_final_mask` AND check (mirrors cpp
// `ParallelCompletionHelper::areAllRegionsInFinal`, T3-inlined), raising
// done.state.s1p1 → pass; the 1 s safety-net never fires.
//
// The runner here is the standard C11 boilerplate; the eventless transition
// chain (s1p111→s1p11final + s1p121→s1p12final) drains to quiescence
// before the 1 s safety-net `<send delay="1s" event="timeout"/>` is
// pumped, so simple `_run` reaches pass without `_tick` polling.

#include <stdio.h>

#include "../require_started.h"
#include "test417_sm.h"

int main(void) {
    test417_t sm;
    REQUIRE_STARTED(test417_init(&sm), TEST417_START_OK);
    test417_run(&sm);

    int rc = test417_ended_in(&sm, TEST417_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test417: FAIL — active = 0x%08x\n", (unsigned)test417_active_states(&sm));
    }
    test417_destroy(&sm);
    return rc;
}
