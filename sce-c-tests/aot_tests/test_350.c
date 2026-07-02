// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test350 — C11 AOT runner.
//
// W3C SCXML 6.2: <send target=...> can deliver to the SAME session by using the session ID as target — the targetexpr
// arm's self-session URI clause (3922f9ab) routes through raise_external; the round-tripped event drives s0→s1→pass
// before the 5 s safety-net timeout.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test350_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test350_t sm;
    test350_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test350_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test350: TIMEOUT — active = 0x%08x\n", (unsigned)test350_active_states(&sm));
            test350_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test350_tick(&sm);
    }

    int rc = test350_in_state(&sm, TEST350_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test350: FAIL — active = 0x%08x\n", (unsigned)test350_active_states(&sm));
    }
    test350_destroy(&sm);
    return rc;
}
