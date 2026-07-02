// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test276 — C11 AOT runner.
//
// Fixture (resources/276/test276.txml):
//   <state id="s0">
//     <invoke type="scxml" src="file:test276sub1.scxml">
//       <param name="Var1" expr="1"/>
//     </invoke>
//     <transition event="event1" target="pass"/>
//     <transition event="event0" target="fail"/>
//   </state>
//
// child (test276sub1):
//   <data id="Var1" expr="0"/>
//   <state id="s0">
//     <transition cond="Var1==1" target="final"><send target="#_parent" event="event1"/></transition>
//     <transition target="final"><send target="#_parent" event="event0"/></transition>
//   </state>
//
// W3C 6.4.1 <param> override-default semantics: parent's <param Var1=1>
// overrides the child's <data id="Var1" expr="0"> default. Child's
// `Var1==1` cond fires, dispatches `event1` → parent reaches pass.
// If the override path were a silent-broken hook, Var1 would stay 0,
// child would dispatch `event0`, and the parent would route to fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test276_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test276_t sm;
    test276_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test276_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test276: TIMEOUT — active = 0x%08x\n", (unsigned)test276_active_states(&sm));
            test276_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test276_step(&sm);
    }

    int rc = test276_in_state(&sm, TEST276_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test276: FAIL — active = 0x%08x\n", (unsigned)test276_active_states(&sm));
    }
    test276_destroy(&sm);
    return rc;
}
