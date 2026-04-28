// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test225 — C11 AOT runner.
//
// Fixture (resources/225/test225.txml):
//   <data id="Var1"/>
//   <data id="Var2"/>
//   <state id="s0">
//     <invoke idlocation="Var1">...</invoke>
//     <invoke idlocation="Var2">...</invoke>
//     <transition event="*" target="s1"/>
//   </state>
//   <state id="s1">
//     <transition cond="Var1 === Var2" target="fail"/>
//     <transition target="pass"/>
//   </state>
//
// W3C 6.4.1 uniqueness: when two `<invoke>` elements both auto-
// generate ids in the same state, the platform must produce distinct
// values. C11's `sce_invoke_format_id` distinguishes them via the
// `invoke_idx` suffix (loop.index0 in the codegen) — Var1 ends with
// `.0`, Var2 ends with `.1`. The s1 cond `Var1 === Var2` is the
// negative path that lands in `fail`; success means the default
// (lower-priority) arm wins, which only happens if the two ids
// differ.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test225_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test225_t sm;
    test225_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test225_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test225: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test225_active_states(&sm));
            test225_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test225_tick(&sm);
    }

    int rc = test225_in_state(&sm, TEST225_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test225: FAIL — active = 0x%08x\n",
                (unsigned)test225_active_states(&sm));
    }
    test225_destroy(&sm);
    return rc;
}
