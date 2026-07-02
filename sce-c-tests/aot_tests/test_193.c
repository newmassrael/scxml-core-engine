// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test193 — C11 AOT runner.
//
// W3C SCXML 6.2: omitting target/targetexpr on <send> still routes through the external queue (not internal) — the s0
// onentry's bare `<send event=internal>` and explicit-type `<send event=event1>` both land externally; the App.D.2
// internal-priority drain pops nothing internal, then external pops in queue order so 'internal' (queued first) drives
// s0→s1 before event1 reaches the s1 pass transition.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test193_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test193_t sm;
    test193_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test193_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test193: TIMEOUT — active = 0x%08x\n", (unsigned)test193_active_states(&sm));
            test193_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test193_tick(&sm);
    }

    int rc = test193_in_state(&sm, TEST193_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test193: FAIL — active = 0x%08x\n", (unsigned)test193_active_states(&sm));
    }
    test193_destroy(&sm);
    return rc;
}
