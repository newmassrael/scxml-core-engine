// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test553 — C11 AOT runner.
//
// W3C 6.2 + C.1: `<send namelist>` evaluation against an undeclared variable
// must raise `error.execution` and stop the dispatch — the receiving
// transition `event="event1" target="fail"` must NOT fire. The s0 onentry
// schedules a 1-second `<send event="timeout"/>` before the failing
// namelist send, so once the namelist eval shorts the rest of onentry the
// timeout still queues normally; after 1 s `_tick` promotes it to the
// external queue and the `<transition event="timeout" target="pass"/>`
// matches first. Same `_tick` polling shape as test579/test580 since the
// pass path depends on the scheduled timeout actually elapsing.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(553) invocation.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test553_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test553_t sm;
    test553_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test553_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test553: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test553_active_states(&sm));
            test553_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test553_tick(&sm);
    }

    int rc = test553_in_state(&sm, TEST553_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test553: FAIL — active = 0x%08x\n",
                (unsigned)test553_active_states(&sm));
    }
    test553_destroy(&sm);
    return rc;
}
