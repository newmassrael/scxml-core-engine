// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test579 — C11 AOT runner.
//
// W3C 3.10 history default content + 6.2 delayed `<send>` success path:
// the s3 transition `event="timeout" target="pass"` only fires when the
// 1-second `<send delay="1s" event="timeout"/>` scheduled at s0 entry
// actually elapses, so unlike the earlier safety-net fixtures (test403a/b
// /c, test404, test405, test580) this runner cannot rely on `_run` and
// must drive `_tick` in a polling loop. Mirrors cpp `ScheduledAotTest::
// runUntilCompletion` — sleep `pollInterval`, call `_tick`, repeat until
// a top-level final state is reached or the timeout cap expires.
//
// W3C SCXML 3.10 + 6.2: history default content runs on first entry only (s0
// re-entry through sh1 restores s03 from snapshot, skipping the default
// `<raise event3>`); the success path requires the 1 s `<send delay>` timeout
// to actually fire, forcing the full polling-driven scheduler

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test579_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test579_t sm;
    REQUIRE_STARTED(test579_init(&sm), TEST579_START_OK);

    /* W3C 6.2: poll cadence 10 ms — same default as cpp `runUntilCompletion`'s
       `pollInterval`. Timeout cap 5 s leaves comfortable headroom over the
       fixture's 1 s scheduled timeout. */
    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test579_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test579: TIMEOUT — active = 0x%08x\n", (unsigned)test579_active_states(&sm));
            test579_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test579_tick(&sm);
    }

    int rc = test579_ended_in(&sm, TEST579_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test579: FAIL — active = 0x%08x\n", (unsigned)test579_active_states(&sm));
    }
    test579_destroy(&sm);
    return rc;
}
