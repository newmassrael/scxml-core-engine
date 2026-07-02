// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test224 — C11 AOT runner.
//
// Fixture (resources/224/test224.txml):
//   <data id="Var1"/>
//   <data id="Var2" expr="'s0.'"/>
//   <state id="s0">
//     <invoke type="http://www.w3.org/TR/scxml/" idlocation="Var1">
//       <content>...inline child reaching final immediately...</content>
//     </invoke>
//     <transition event="*" target="s1"/>
//   </state>
//   <state id="s1">
//     <transition cond="Var1.indexOf(Var2) === 0" target="pass"/>
//     <transition target="fail"/>
//   </state>
//
// W3C 3.12.1 / 6.4.1: pins the auto-generated invoke id format. The
// spec requires `<state_id>.<platform_token>` so the parent can
// recognise which state owned the spawn. C11's `sce_invoke_format_id`
// (backends/c/runtime/include/sce/invoke.h) emits
// `<state_id>.<sm_ptr_hex>.<invoke_idx>`, mirroring cpp's
// `core/InvokeHelper.h`. The cond `Var1.indexOf('s0.') === 0` checks
// that the id was deposited via idlocation lower AND begins with the
// owning state name + '.' separator.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test224_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test224_t sm;
    test224_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test224_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test224: TIMEOUT — active = 0x%08x\n", (unsigned)test224_active_states(&sm));
            test224_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test224_tick(&sm);
    }

    int rc = test224_in_state(&sm, TEST224_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test224: FAIL — active = 0x%08x\n", (unsigned)test224_active_states(&sm));
    }
    test224_destroy(&sm);
    return rc;
}
