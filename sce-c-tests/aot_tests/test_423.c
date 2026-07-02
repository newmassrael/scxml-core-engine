// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test423 — C11 AOT runner.
//
// W3C SCXML 3.13 + App.D.2: external queue drains until a matching transition is found — externalEvent1 has no s0-level
// matching transition so it is popped and discarded, externalEvent2 (1 s delayed) eventually fires and matches the s1
// transition → pass.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test423_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test423_t sm;
    test423_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test423_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test423: TIMEOUT — active = 0x%08x\n", (unsigned)test423_active_states(&sm));
            test423_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test423_tick(&sm);
    }

    int rc = test423_in_state(&sm, TEST423_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test423: FAIL — active = 0x%08x\n", (unsigned)test423_active_states(&sm));
    }
    test423_destroy(&sm);
    return rc;
}
