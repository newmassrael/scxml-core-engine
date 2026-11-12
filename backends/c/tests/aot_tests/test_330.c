// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//
// W3C SCXML 5.10: every dispatched event carries the required `name` field —
// s0's <raise event=foo> (internal queue) and s1's bare <send event=foo>
// (external queue) both round-trip the same name through the existing
// set_current_event lua chunk so the matching event="foo" transitions cover
// both queue paths

#include <stdio.h>

#include "../require_started.h"
#include "test330_sm.h"

int main(void) {
    test330_t sm;
    REQUIRE_STARTED(test330_init(&sm), TEST330_START_OK);
    test330_run(&sm);

    int rc = test330_ended_in(&sm, TEST330_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test330: FAIL — active = 0x%08x\n", (unsigned)test330_active_states(&sm));
    }
    test330_destroy(&sm);
    return rc;
}
