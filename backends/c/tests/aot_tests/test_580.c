// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test580 — C11 AOT runner.
//
// W3C 3.10 (history pseudo-state never appears in the active configuration)
// crossed with 3.4 (parallel) and 6.2 (delayed `<send>` safety-net). The
// success path is purely eventless inside p1's two regions:
//   1. s1's initial transition enters sh1 (no record) → default leaf s11
//   2. s11's eventless `<transition target="s12"/>` advances to s12
//   3. s1's eventless `<transition cond="Var1==0" target="sh1"/>` exits s12
//      (onexit `Var1++`) and re-enters via history (recorded snapshot s12)
//   4. s1's eventless `<transition cond="Var1==1" target="pass"/>` reaches
//      the top-level final.
// The 2-second `<send delay="2s" event="timeout"/>` is a safety-net whose
// `<transition event="timeout" target="fail"/>` only fires if the eventless
// chain stalls. Polling via `_tick` rather than `_run` exercises the same
// scheduler that test579 depends on, keeping a single code shape across
// the success-path-timeout and safety-net-timeout fixtures.
//
// W3C SCXML 3.10 + 3.4: history pseudo-state never enters the active
// configuration even across <parallel> regions — `In('sh1')` cond stays false
// through the entire run, and the 2 s safety-net timeout never fires because
// the eventless chain reaches `pass` first

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test580_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test580_t sm;
    REQUIRE_STARTED(test580_init(&sm), TEST580_START_OK);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test580_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test580: TIMEOUT — active = 0x%08x\n", (unsigned)test580_active_states(&sm));
            test580_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test580_tick(&sm);
    }

    int rc = test580_ended_in(&sm, TEST580_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test580: FAIL — active = 0x%08x\n", (unsigned)test580_active_states(&sm));
    }
    test580_destroy(&sm);
    return rc;
}
