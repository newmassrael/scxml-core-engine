// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test347 — C11 AOT runner.
//
// Fixture (resources/347/test347.txml): child onentry sends
// `<send type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
// target="#_parent" event="childToParent"/>`. Parent's s02 onentry
// reciprocates with `<send type="...SCXMLEventProcessor"
// target="#_child" event="parentToChild"/>`. Child receives
// parentToChild → subFinal → done.invoke → parent reaches pass.
//
// Pins: explicit `type="...SCXMLEventProcessor"` is the same as the
// default no-type send for both `#_parent` (child→parent) and
// `#_<invokeid>` (parent→child) — the existing send-action lower
// already accepts this URI in its `send_type` predicate.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test347_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test347_t sm;
    test347_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test347_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test347: TIMEOUT — active = 0x%08x\n", (unsigned)test347_active_states(&sm));
            test347_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test347_tick(&sm);
    }

    int rc = test347_in_state(&sm, TEST347_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test347: FAIL — active = 0x%08x\n", (unsigned)test347_active_states(&sm));
    }
    test347_destroy(&sm);
    return rc;
}
