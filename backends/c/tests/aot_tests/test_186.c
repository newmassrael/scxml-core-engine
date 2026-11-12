// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test186 — C11 AOT runner.
//
// W3C 5.10 + 6.2 (send-time param eval): the onentry of s0 schedules
// `<send event="event1" delay="1s"><param name="aParam" expr="Var1"/></send>`
// while Var1=1, then immediately reassigns Var1=2. After the 1 s timer
// fires, the receiving transition reads `_event.data.aParam` into Var2 —
// per spec the captured value must be the send-time snapshot (1), not
// the delivery-time evaluation (2). The cond `Var2 == 1` in s1 then
// routes to pass; a delivery-time evaluation would yield Var2=2 and
// trip the s1 catch-all to fail. The snapshot is stored on the scheduled
// entry as its event's JSON `data[]`, so nothing sent in between can
// replace it.
//
// W3C SCXML 5.10 + 6.2: delayed <send><param expr="Var1"/></send> snapshots
// Var1=1 at send time even though the surrounding onentry reassigns Var1=2
// before the 1 s timer fires — the payload captured at send time is stored on
// the scheduled entry as its event's JSON `data[]`, and the _tick dispatch
// raises the event with it, so the receiving transition reads aParam=1,
// routing to pass

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test186_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test186_t sm;
    REQUIRE_STARTED(test186_init(&sm), TEST186_START_OK);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test186_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test186: TIMEOUT — active = 0x%08x\n", (unsigned)test186_active_states(&sm));
            test186_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test186_tick(&sm);
    }

    int rc = test186_ended_in(&sm, TEST186_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test186: FAIL — active = 0x%08x\n", (unsigned)test186_active_states(&sm));
    }
    test186_destroy(&sm);
    return rc;
}
