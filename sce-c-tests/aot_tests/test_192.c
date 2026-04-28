// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test192 — C11 AOT runner.
//
// Fixture (resources/192/test192.txml):
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="5s"/></onentry>
//     <invoke id="invokedChild"><content>
//       <scxml initial="sub0">
//         <state id="sub0">
//           <onentry>
//             <send event="childToParent" target="#_parent"/>
//             <send event="timeout" delay="3s"/>
//           </onentry>
//           <transition event="parentToChild" target="subFinal">
//             <send target="#_parent" event="eventReceived"/>
//           </transition>
//           <transition event="timeout" target="subFinal"/>
//         </state>
//         <final id="subFinal"/>
//       </scxml>
//     </content></invoke>
//     <transition event="timeout" target="fail"/>
//     <transition event="done.invoke" target="fail"/>
//     <state id="s01">
//       <transition event="childToParent" target="s02">
//         <send target="#_invokedChild" event="parentToChild"/>
//       </transition>
//     </state>
//     <state id="s02"><transition event="eventReceived" target="pass"/></state>
//   </state>
//
// Pins: (1) parent's `<send target="#_<invokeid>">` lower resolves the
// `#_invokedChild` literal to the matching invoke at codegen time and
// dispatches via the child's public `_raise_external_by_name` shim.
// (2) `_step`'s `_drive_active_children` call drains the child's
// external queue on the same macrostep so the round-trip
// childToParent → parentToChild → eventReceived completes within the
// 5 s parent-side timeout.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test192_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test192_t sm;
    test192_init(&sm);

    const uint64_t timeout_ms = 7000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test192_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test192: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test192_active_states(&sm));
            test192_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test192_tick(&sm);
    }

    int rc = test192_in_state(&sm, TEST192_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test192: FAIL — active = 0x%08x\n",
                (unsigned)test192_active_states(&sm));
    }
    test192_destroy(&sm);
    return rc;
}
