// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test236 — C11 AOT runner.
//
// Fixture (resources/236/test236.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke type="..."><content>
//       <scxml initial="subFinal">
//         <final id="subFinal">
//           <onexit><send target="#_parent" event="childToParent"/></onexit>
//         </final>
//       </scxml>
//     </content></invoke>
//     <transition event="childToParent" target="s1"/>
//     <transition event="done.invoke" target="fail"/>
//   </state>
//   <state id="s1">
//     <transition event="done.invoke" target="s2"/>
//     <transition event="*" target="fail"/>
//   </state>
//   <state id="s2">
//     <transition event="timeout" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// Pins W3C 3.4 + 6.4 verdict-before-completion ordering. Child is a
// pure top-level `<final>` so `is_in_final_state` returns true at
// init time. Parent's invoke helper runs `_finalize_session` (which
// fires subFinal's `<onexit>` → childToParent → parent's external
// queue) BEFORE raising `done.invoke`. Parent transitions s0→s1 on
// childToParent, then s1→s2 on done.invoke, then s2→pass on the 2 s
// timeout. Without `_finalize_session` done.invoke arrives first →
// s0→fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test236_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test236_t sm;
    test236_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test236_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test236: TIMEOUT — active = 0x%08x\n", (unsigned)test236_active_states(&sm));
            test236_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test236_tick(&sm);
    }

    int rc = test236_in_state(&sm, TEST236_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test236: FAIL — active = 0x%08x\n", (unsigned)test236_active_states(&sm));
    }
    test236_destroy(&sm);
    return rc;
}
