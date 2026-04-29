// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test253 — C11 AOT runner.
//
// Fixture (resources/253/test253.txml): bidirectional `#_parent` /
// `#_<invokeid>` event flow + `_event.origintype` round-trip. Child
// sends `childRunning` to `#_parent`; parent's s01 captures
// `_event.origintype` into Var1 then transitions to s02. s02's
// cond `Var1 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'`
// must hold for parent to dispatch `parentToChild` to `#_foo`. Child's
// sub1 transition assigns `_event.origintype` into Var2 then takes
// the URI-cond-true branch, sending `success` back. Parent reaches
// pass on success.
//
// Pins: `_event.origintype` is the SCXMLEventProcessor URI on every
// external-queue pop (set in `process_event_queues`'s external-queue
// branch via `_pending_event_origintype = ...`). Both directions of
// the cross-machine routing — child→parent (`#_parent`) and parent→
// child (`#_foo`) — go through external-queue raises and so receive
// the same origintype seed; the URI cond holds at both sites.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test253_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test253_t sm;
    test253_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test253_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test253: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test253_active_states(&sm));
            test253_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test253_tick(&sm);
    }

    int rc = test253_in_state(&sm, TEST253_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test253: FAIL — active = 0x%08x\n",
                (unsigned)test253_active_states(&sm));
    }
    test253_destroy(&sm);
    return rc;
}
