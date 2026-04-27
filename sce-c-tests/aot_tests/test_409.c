// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test409 — C11 AOT runner.
//
// W3C SCXML 3.4 + 5.9.2: when s01's `<onexit>` runs, s011 must already
// be removed from the configuration so the cond `In('s011')` reads
// false — therefore event1 is *not* raised, only the s0-onentry-scheduled
// `<send delay="1s" event="timeout"/>` ever fires, and the s0 transition
// `event="timeout" target="pass"` decides the verdict. Timeout is the
// **success path** here (unlike the safety-net pattern in test411), so
// the runner cannot rely on `_run` and must poll `_tick` until either
// the schedulered fire elapses or a top-level final is reached. Mirrors
// cpp `ScheduledAotTest::runUntilCompletion` cadence.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test409_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test409_t sm;
    test409_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test409_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test409: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test409_active_states(&sm));
            test409_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test409_tick(&sm);
    }

    int rc = test409_in_state(&sm, TEST409_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test409: FAIL — active = 0x%08x\n",
                (unsigned)test409_active_states(&sm));
    }
    test409_destroy(&sm);
    return rc;
}
