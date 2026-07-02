// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test174 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send typeexpr="...">` evaluates the send type string at
// send time. The fixture starts with Var1 unbound, then in onentry sets
// Var1 = 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor' (the only type
// the C11 backend currently supports), so the typeexpr eval matches and
// the send proceeds as a normal external dispatch — the receiving
// transition then matches event1 → pass. If the eval used the initial
// (undefined) value the chunk would fail and fall back to the default
// SCXMLEventProcessor URI per cpp's line 47, still landing on pass.

#include <stdio.h>

#include "test174_sm.h"

int main(void) {
    test174_t sm;
    test174_init(&sm);
    test174_run(&sm);

    int rc = test174_in_state(&sm, TEST174_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test174: FAIL — active = 0x%08x\n", (unsigned)test174_active_states(&sm));
    }
    test174_destroy(&sm);
    return rc;
}
