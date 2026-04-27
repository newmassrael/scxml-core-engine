// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test185 — C11 AOT runner.
//
// W3C SCXML 6.2: <send> respects the delay specification — onentry sends event2 with delay=1s then event1 immediately; the immediate event1 fires first (routes s0→s1), then the 1s timer fires event2 → pass. If the scheduler ignored delay, event2 would land first against the s0 wildcard → fail. Tests scheduled_push fire_time_ms-keyed sort (옵션 σ).

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test185_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test185_t sm;
    test185_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test185_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test185: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test185_active_states(&sm));
            test185_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test185_tick(&sm);
    }

    int rc = test185_in_state(&sm, TEST185_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test185: FAIL — active = 0x%08x\n",
                (unsigned)test185_active_states(&sm));
    }
    test185_destroy(&sm);
    return rc;
}
