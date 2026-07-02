// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test247 — C11 AOT runner.
//
// Fixture (resources/247/test247.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke type="http://www.w3.org/TR/scxml/">
//       <content>
//         <scxml initial="subFinal"><final id="subFinal"/></scxml>
//       </content>
//     </invoke>
//     <transition event="done.invoke" target="pass"/>
//     <transition event="timeout" target="fail"/>
//   </state>
//
// Polling-driven shape mirrors test175/579: the 2 s `<send delay>` is
// the safety-net (target=fail), so the runner needs `_tick` calls to
// pump the scheduled queue. The success path is independent of the
// timeout — child SM reaches its top-level <final> synchronously inside
// `_init`, so the parent's macrostep-end `execute_pending_invokes`
// raises done.invoke onto the external queue during the very first
// stabilisation pass; the next process_event_queues drain dispatches
// done.invoke and drives s0→pass before the polling loop ever returns
// from its first sleep. The polling loop is still required because the
// test harness must still be able to observe the timeout-routes-to-fail
// path if the success path regresses.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test247_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test247_t sm;
    test247_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test247_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test247: TIMEOUT — active = 0x%08x\n", (unsigned)test247_active_states(&sm));
            test247_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test247_tick(&sm);
    }

    int rc = test247_in_state(&sm, TEST247_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test247: FAIL — active = 0x%08x\n", (unsigned)test247_active_states(&sm));
    }
    test247_destroy(&sm);
    return rc;
}
