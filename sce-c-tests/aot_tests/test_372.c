// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test372 — C11 AOT runner.
//
// W3C SCXML 3.7 + 3.13: done.state.parent fires only AFTER the final state's onentry runs to completion — Var1 is
// assigned 2 in onentry, then done.state.s0 raises; the receiving cond `Var1 == 2` matches before any later onentry
// block could shift Var1 to 3.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test372_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test372_t sm;
    test372_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test372_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test372: TIMEOUT — active = 0x%08x\n", (unsigned)test372_active_states(&sm));
            test372_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test372_tick(&sm);
    }

    int rc = test372_in_state(&sm, TEST372_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test372: FAIL — active = 0x%08x\n", (unsigned)test372_active_states(&sm));
    }
    test372_destroy(&sm);
    return rc;
}
