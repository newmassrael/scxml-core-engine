// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test178 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send event="event1">` with two `<param name="Var1">`
// declarations of the same name must dispatch event1 with both key/value
// pairs preserved. The W3C corpus marks this as a manual test because
// the SCXMLEventProcessor does not specify the wire format, so the
// per-binding observation lives in `<log expr="_event.raw">` for human
// inspection. The automated success criterion mirrors cpp `Test178.h` —
// reach the `final` state (any other arrival routes to the wildcard
// `fail` final). The duplicate-param emit shape is exercised at codegen
// time by the params loop in the bare-external send arm; the event
// dispatches verbatim and the receiving transition fires.

#include <stdio.h>

#include "test178_sm.h"

int main(void) {
    test178_t sm;
    test178_init(&sm);
    test178_run(&sm);

    int rc = test178_in_state(&sm, TEST178_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test178: FAIL — active = 0x%08x\n", (unsigned)test178_active_states(&sm));
    }
    test178_destroy(&sm);
    return rc;
}
