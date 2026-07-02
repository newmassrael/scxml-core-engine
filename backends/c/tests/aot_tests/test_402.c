// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test402 — C11 AOT runner.
//
// W3C SCXML 5.10: errors are 'like any other event' — an empty-location <assign> raises error.execution, but the
// surrounding entry-action chain continues executing (errors do not abort the run); the receiving transition picks up
// the error and routes to pass before the 1 s safety-net (target=fail) fires.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test402_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test402_t sm;
    test402_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test402_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test402: TIMEOUT — active = 0x%08x\n", (unsigned)test402_active_states(&sm));
            test402_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test402_tick(&sm);
    }

    int rc = test402_in_state(&sm, TEST402_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test402: FAIL — active = 0x%08x\n", (unsigned)test402_active_states(&sm));
    }
    test402_destroy(&sm);
    return rc;
}
