// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test416 — C11 AOT runner.
//
// W3C SCXML 3.7: top-level final state halts execution — entering a child of the top-level final triggers
// `is_in_final_state` to return true, so process_event_queues returns immediately without firing the safety-net timeout
// (target=fail). Routes to pass via eventless chain.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test416_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test416_t sm;
    test416_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test416_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test416: TIMEOUT — active = 0x%08x\n", (unsigned)test416_active_states(&sm));
            test416_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test416_tick(&sm);
    }

    int rc = test416_in_state(&sm, TEST416_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test416: FAIL — active = 0x%08x\n", (unsigned)test416_active_states(&sm));
    }
    test416_destroy(&sm);
    return rc;
}
