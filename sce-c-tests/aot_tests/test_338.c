// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test338 — C11 AOT runner.
//
// Fixture (resources/338/test338.txml):
//   <data id="Var1"/><data id="Var2"/>
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke idlocation="Var1" type="..."><content>
//       <scxml name="machineName">
//         <final id="sub0">
//           <onentry><send target="#_parent" event="event1"/></onentry>
//         </final>
//       </scxml>
//     </content></invoke>
//     <transition event="event1" target="s1">
//       <assign location="Var2" expr="_event.invokeid"/>
//     </transition>
//   </state>
//   <state id="s1">
//     <transition cond="Var1 === Var2" target="pass"/>
//     <transition target="fail"/>
//   </state>
//
// Pins: (1) onentry-stamp ordering — `<onentry>` of `<final>` runs the
// `<send target="#_parent">` during the child's entry walk; live
// parent_dispatch is required. (2) `_event.invokeid` plumbing
// (test228 surface) — Var2 captures the child's auto-generated id, must
// equal idlocation-bound Var1.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test338_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test338_t sm;
    test338_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test338_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test338: TIMEOUT — active = 0x%08x\n", (unsigned)test338_active_states(&sm));
            test338_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test338_tick(&sm);
    }

    int rc = test338_in_state(&sm, TEST338_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test338: FAIL — active = 0x%08x\n", (unsigned)test338_active_states(&sm));
    }
    test338_destroy(&sm);
    return rc;
}
