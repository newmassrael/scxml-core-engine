// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test244 — C11 AOT runner.
//
// Fixture (resources/244/test244.txml):
//   <data id="Var1" expr="1"/>
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke ... namelist="Var1">
//       <content>...child with <data id="Var1" expr="0">...</content>
//     </invoke>
//     <transition event="success" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.4.1 namelist single-value transfer: parent's Var1=1 transfers
// via namelist to the child's Var1 slot, overriding the child's
// default. Child's `Var1==1` cond fires, dispatches `success` → pass.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test244_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test244_t sm;
    test244_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test244_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test244: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test244_active_states(&sm));
            test244_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test244_tick(&sm);
    }

    int rc = test244_in_state(&sm, TEST244_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test244: FAIL — active = 0x%08x\n",
                (unsigned)test244_active_states(&sm));
    }
    test244_destroy(&sm);
    return rc;
}
