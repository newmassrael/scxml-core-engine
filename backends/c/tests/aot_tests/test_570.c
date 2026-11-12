// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test570 — C11 AOT runner.
//
// W3C SCXML 3.4 + 3.7.1: parallel completion ordering — p0 onentry raises
// e1+e2, e1 drives p0s11→p0s1final (raising done.state.p0s1, parallel
// completion check finds p0s2 still non-final → no done.state.p0), the
// done.state.p0s1 transition's `<assign Var1=1>` lands, e2 then drives
// p0s21→p0s2final (raising done.state.p0s2, completion check now finds both
// regions final → raises done.state.p0), the done.state.p0s2 transition exits
// to s1, and s1's `done.state.p0` cond `Var1==1` routes to pass — verifies
// the second-compound-final entry is the trigger (not the first) and that
// `<transition>` body executes between done events; 2 s safety-net never
// fires.
//
// The runner here is the standard C11 boilerplate; raise e1/e2 + the
// done.state internal chain drain to quiescence before the 2 s
// safety-net `<send delay="2s" event="timeout"/>` is pumped, so simple
// `_run` reaches pass without `_tick` polling.

#include <stdio.h>

#include "../require_started.h"
#include "test570_sm.h"

int main(void) {
    test570_t sm;
    REQUIRE_STARTED(test570_init(&sm), TEST570_START_OK);
    test570_run(&sm);

    int rc = test570_ended_in(&sm, TEST570_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test570: FAIL — active = 0x%08x\n", (unsigned)test570_active_states(&sm));
    }
    test570_destroy(&sm);
    return rc;
}
