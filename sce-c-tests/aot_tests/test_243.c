// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test243 — C11 AOT runner.
//
// Fixture (resources/243/test243.txml):
//   <data id="Var1" expr="1"/>
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke ...>
//       <param name="Var1" expr="1"/>
//       <content>...child with <data id="Var1" expr="0">...</content>
//     </invoke>
//     <transition event="success" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.4.1 <param> single-value transfer: parent's <param Var1=1>
// overrides the child's <data id="Var1" expr="0"> default. Child's
// `Var1==1` cond fires, dispatches `success` via #_parent → pass.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test243_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test243_t sm;
    test243_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test243_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test243: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test243_active_states(&sm));
            test243_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test243_tick(&sm);
    }

    int rc = test243_in_state(&sm, TEST243_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test243: FAIL — active = 0x%08x\n",
                (unsigned)test243_active_states(&sm));
    }
    test243_destroy(&sm);
    return rc;
}
