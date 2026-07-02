// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test330 — C11 AOT runner.
//
// W3C SCXML 5.10: every dispatched event carries the required `name`
// field on `_event` regardless of source. The fixture's s0 onentry
// raises `foo` (internal queue), s1 onentry sends `foo` (external
// queue); each transition matches `event="foo"` so both dispatch paths
// must round-trip the same name. Any wildcard fall-through to fail
// indicates the event-name binding (set_current_event) leaked between
// internal and external pop sites.

#include <stdio.h>

#include "test330_sm.h"

int main(void) {
    test330_t sm;
    test330_init(&sm);
    test330_run(&sm);

    int rc = test330_in_state(&sm, TEST330_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test330: FAIL — active = 0x%08x\n", (unsigned)test330_active_states(&sm));
    }
    test330_destroy(&sm);
    return rc;
}
