// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test191 — C11 AOT runner.
//
// Fixture (resources/191/test191.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="5s"/></onentry>
//     <invoke type="scxml"><content>
//       <scxml initial="sub0">
//         <state id="sub0">
//           <onentry><send event="childToParent" target="#_parent"/></onentry>
//           <transition target="subFinal"/>
//         </state>
//         <final id="subFinal"/>
//       </scxml>
//     </content></invoke>
//     <transition event="childToParent" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.4 onentry-stamp ordering: the child's `<onentry>` runs during
// `enter_state_recursive` inside `_init_with_parent`. With parent_dispatch
// stamped BEFORE the entry walk, the send routes to the parent's external
// queue and parent's `childToParent` transition fires → s0 → pass. Without
// the stamp the send drops under a NULL guard, child reaches subFinal,
// done.invoke routes to `*` → fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test191_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test191_t sm;
    test191_init(&sm);

    const uint64_t timeout_ms = 7000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test191_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test191: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test191_active_states(&sm));
            test191_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test191_tick(&sm);
    }

    int rc = test191_in_state(&sm, TEST191_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test191: FAIL — active = 0x%08x\n",
                (unsigned)test191_active_states(&sm));
    }
    test191_destroy(&sm);
    return rc;
}
