// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test333 — C11 AOT runner.
//
// W3C SCXML 5.10.1: a bare-external `<send event="foo"/>` does NOT
// populate `_event.sendid` for the receiver — the meta field is zero
// and `set_current_event`'s lua chunk binds `_event.sendid` to nil. The
// fixture's transition is a plain `event="foo"` match (no sendid cond),
// so it just verifies the bare-external dispatch reaches the receiver
// regardless of metadata population. Doubles as a regression guard for
// the set_current_event signature change to `event_with_meta_t *`.
//
// W3C SCXML 5.10.1: bare-external <send event=foo> leaves the
// event_with_meta_t.send_id field zero so set_current_event binds
// `_event.sendid` to nil — the receiving transition is a plain event="foo"
// match (no sendid cond), confirming the metadata extension path doesn't
// break non-sendid-bearing events

#include <stdio.h>

#include "../require_started.h"
#include "test333_sm.h"

int main(void) {
    test333_t sm;
    REQUIRE_STARTED(test333_init(&sm), TEST333_START_OK);
    test333_run(&sm);

    int rc = test333_ended_in(&sm, TEST333_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test333: FAIL — active = 0x%08x\n", (unsigned)test333_active_states(&sm));
    }
    test333_destroy(&sm);
    return rc;
}
