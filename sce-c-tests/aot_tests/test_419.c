// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test419 — C11 AOT runner.
//
// W3C SCXML 3.13 + App.D.2: eventless transitions take precedence over
// event-bound ones at the same source. The fixture's s1 onentry raises
// internalEvent (internal queue) and sends externalEvent (external
// queue), then declares two transitions: `<transition event="*"
// target="fail"/>` and unconditional `<transition target="pass"/>`.
// The init-tail macrostep loop calls `check_eventless_transitions`
// before `process_event_queues`, so the eventless `target="pass"`
// fires while both queued events still wait. Either ordering bug
// (internal pop before eventless or wildcard preempting eventless)
// routes to fail.

#include <stdio.h>

#include "test419_sm.h"

int main(void) {
    test419_t sm;
    test419_init(&sm);
    test419_run(&sm);

    int rc = test419_in_state(&sm, TEST419_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test419: FAIL — active = 0x%08x\n", (unsigned)test419_active_states(&sm));
    }
    test419_destroy(&sm);
    return rc;
}
