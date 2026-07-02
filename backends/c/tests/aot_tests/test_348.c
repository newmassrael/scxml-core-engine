// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test348 — C11 AOT runner.
//
// W3C SCXML 6.2: explicit `type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"`
// is the default Event I/O Processor and MUST be treated identically
// to a bare `<send event=...>`. The fixture's s0 onentry sends
// `s0Event` with the literal default-type URI; with the carve-out
// active the SCXMLEventProcessor literal collapses to bare-external
// dispatch and the s0→pass transition fires. Without the carve-out
// the `#error` guard would fail compilation.

#include <stdio.h>

#include "test348_sm.h"

int main(void) {
    test348_t sm;
    test348_init(&sm);
    test348_run(&sm);

    int rc = test348_in_state(&sm, TEST348_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test348: FAIL — active = 0x%08x\n", (unsigned)test348_active_states(&sm));
    }
    test348_destroy(&sm);
    return rc;
}
