// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test237 — C11 AOT runner.
//
// Fixture (resources/237/test237.txml):
//   <state id="s0">
//     <onentry><send event="timeout1" delay="1s"/></onentry>
//     <invoke><content>
//       <scxml initial="sub0">
//         <state id="sub0">
//           <onentry><send event="timeout" delay="2s"/></onentry>
//           <transition event="timeout" target="subFinal"/>
//         </state>
//         <final id="subFinal"/>
//       </scxml>
//     </content></invoke>
//     <transition event="timeout1" target="s1"/>
//   </state>
//   <state id="s1">
//     <onentry><send event="timeout2" delay="1.5s"/></onentry>
//     <transition event="done.invoke" target="fail"/>
//     <transition event="*" target="pass"/>
//   </state>
//
// W3C SCXML 6.4 cancel-on-state-exit. Child schedules a 2s `timeout`
// in its own session; parent's 1s `timeout1` fires first and exits s0.
// On s0 exit, the existing onexit hook calls `destroy_active_children`
// which deallocates the child SM (zeroing its `scheduled_queue`). The
// child's 2s entry never promotes — its `_tick` is never reached
// because `child_active = false`. Parent's s1 onentry schedules a 1.5s
// `timeout2`; with no `done.invoke` arriving (child was cancelled),
// the wildcard `event="*"` arm catches `timeout2` and routes to pass.
//
// Builds on test252's cancel-path infrastructure (parent_dispatch NULL
// stamp + destroy) — the difference is that test252 cancels via a
// transition out of the invoking state's *parent compound*, whereas
// test237 cancels via the invoking state itself exiting; both routes
// converge on the same `destroy_active_children` cleanup.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test237_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test237_t sm;
    test237_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test237_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test237: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test237_active_states(&sm));
            test237_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test237_tick(&sm);
    }

    int rc = test237_in_state(&sm, TEST237_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test237: FAIL — active = 0x%08x\n",
                (unsigned)test237_active_states(&sm));
    }
    test237_destroy(&sm);
    return rc;
}
