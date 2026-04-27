// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test208 — C11 AOT runner with polling-driven _tick.
//
// W3C SCXML 6.3: `<cancel sendid="foo"/>` removes the matching entry
// from the scheduled queue before its fire_time elapses. Onentry queues
// event1 (1s, id="foo"), event2 (1.5s), then cancels foo — so event2
// fires first → s0 transition → pass. If cancel were a no-op, event1
// would arrive before event2 and the wildcard `*` would route to fail.
// Same polling pattern as test579/580/175.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test208_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test208_t sm;
    test208_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test208_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test208: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test208_active_states(&sm));
            test208_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test208_tick(&sm);
    }

    int rc = test208_in_state(&sm, TEST208_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test208: FAIL — active = 0x%08x\n",
                (unsigned)test208_active_states(&sm));
    }
    test208_destroy(&sm);
    return rc;
}
