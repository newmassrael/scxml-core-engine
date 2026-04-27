// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test311 — C11 AOT runner.
//
// W3C SCXML 5.4: `<assign location="">` with an empty location attribute raises error.execution — the lua_assign macro's empty-location guard (8b1bb1e9) emits the error and the receiving error.execution transition matches before the safety-net 1 s timeout fires.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test311_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test311_t sm;
    test311_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test311_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test311: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test311_active_states(&sm));
            test311_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test311_tick(&sm);
    }

    int rc = test311_in_state(&sm, TEST311_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test311: FAIL — active = 0x%08x\n",
                (unsigned)test311_active_states(&sm));
    }
    test311_destroy(&sm);
    return rc;
}
