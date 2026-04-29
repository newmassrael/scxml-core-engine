// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test252 — C11 AOT runner.
//
// Fixture (resources/252/test252.txml):
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="1s"/></onentry>
//     <transition event="timeout" target="pass"/>
//     <transition event="childToParent" target="fail"/>
//     <transition event="done.invoke" target="fail"/>
//     <state id="s01">
//       <onentry><send event="foo"/></onentry>
//       <invoke><content>
//         <scxml initial="sub0">
//           <state id="sub0">
//             <onentry><send event="timeout" delay=".5"/></onentry>
//             <transition event="timeout" target="subFinal"/>
//             <onexit><send target="#_parent" event="childToParent"/></onexit>
//           </state>
//           <final id="subFinal"/>
//         </scxml>
//       </content></invoke>
//       <transition event="foo" target="s02"/>
//     </state>
//     <state id="s02"/>
//   </state>
//
// Pins W3C 6.4 cancellation-drop semantics. Parent's s01 onentry
// raises `foo`; the same macrostep transitions s01→s02, exiting s01
// and canceling the invoke. The cancel path NULLs
// `child.parent_dispatch` BEFORE `child_destroy`, so sub0's
// `<onexit><send target="#_parent">` (which would otherwise fire on
// destroy as part of the active-config exit walk) drops at the NULL
// guard. Parent's 1 s timeout fires unopposed → pass. If the cancel
// path skipped the NULL stamp, childToParent would route to fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test252_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test252_t sm;
    test252_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test252_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test252: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test252_active_states(&sm));
            test252_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test252_tick(&sm);
    }

    int rc = test252_in_state(&sm, TEST252_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test252: FAIL — active = 0x%08x\n",
                (unsigned)test252_active_states(&sm));
    }
    test252_destroy(&sm);
    return rc;
}
