// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test352 — C11 AOT runner.
//
// W3C SCXML 5.10.1: _event.origintype is bound to the SCXMLEventProcessor URI literal on every external pop — assign
// Var1=_event.origintype reads back the URI and the cond matches; same τ binding test336 covers (3922f9ab).

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test352_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test352_t sm;
    test352_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test352_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test352: TIMEOUT — active = 0x%08x\n", (unsigned)test352_active_states(&sm));
            test352_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test352_tick(&sm);
    }

    int rc = test352_in_state(&sm, TEST352_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test352: FAIL — active = 0x%08x\n", (unsigned)test352_active_states(&sm));
    }
    test352_destroy(&sm);
    return rc;
}
