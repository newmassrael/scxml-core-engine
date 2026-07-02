// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test210 — C11 AOT runner with polling-driven _tick.
//
// W3C SCXML 6.3: `<cancel sendidexpr="Var1"/>` evaluates Var1 at cancel
// time (Var1 reassigned 'foo' before cancel) → cancels event1 (id=foo)
// → event2 fires first → pass. The runtime sendidexpr eval routes
// through `lua_eval_eventexpr` (the consumer-agnostic to-string helper)
// then forwards into the same `scheduled_cancel` static helper used by
// the literal-sendid arm.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test210_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test210_t sm;
    test210_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test210_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test210: TIMEOUT — active = 0x%08x\n", (unsigned)test210_active_states(&sm));
            test210_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test210_tick(&sm);
    }

    int rc = test210_in_state(&sm, TEST210_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test210: FAIL — active = 0x%08x\n", (unsigned)test210_active_states(&sm));
    }
    test210_destroy(&sm);
    return rc;
}
