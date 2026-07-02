// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test175 — C11 AOT runner with polling-driven _tick.
//
// W3C SCXML 6.2: `<send delayexpr="..." event="event2"/>` evaluates Var1
// (= '1s' after the onentry assign) at send time and parses it into 1000
// ms; a sibling `<send delay=".5" event="event1"/>` schedules event1 at
// 500 ms. Both events queue onto the scheduled queue; tick polling fires
// event1 first → s0 transitions to s1 → s1 transitions on event2 → pass.
// If delayexpr used the initial Var1 ('0s') event2 would fire immediately
// and the s0 wildcard transition would route to fail.
//
// Same polling pattern as test579/580 — POSIX nanosleep(10ms) between
// `_tick(sm)` calls, capped at 5 seconds.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test175_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test175_t sm;
    test175_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test175_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test175: TIMEOUT — active = 0x%08x\n", (unsigned)test175_active_states(&sm));
            test175_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test175_tick(&sm);
    }

    int rc = test175_in_state(&sm, TEST175_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test175: FAIL — active = 0x%08x\n", (unsigned)test175_active_states(&sm));
    }
    test175_destroy(&sm);
    return rc;
}
