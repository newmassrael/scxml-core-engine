// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test190 — C11 AOT runner.
//
// W3C SCXML 6.2 + C.1.5: `<send targetexpr=Var1>` where Var1 holds a
// session-URI-shaped string (`'#_scxml_'`) routes onto the EXTERNAL
// queue per the W3C spec — cpp `actions/send.jinja2` falls through any
// non-invalid / non-unreachable / non-internal target to
// `engine.raiseExternal`, and the C11 mirror does the same. The fixture
// then proves W3C App.D.2 ordering: the s0 onentry queues event2 onto
// the external queue (via targetexpr), event1 onto the internal queue
// (`<raise>`), and timeout onto the external queue (bare `<send>`); the
// internal-first drain pops event1 → s0→s1, then the external pop yields
// event2 → s1→pass before the timeout fall-through.

#include <stdio.h>

#include "test190_sm.h"

int main(void) {
    test190_t sm;
    test190_init(&sm);
    test190_run(&sm);

    int rc = test190_in_state(&sm, TEST190_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test190: FAIL — active = 0x%08x\n", (unsigned)test190_active_states(&sm));
    }
    test190_destroy(&sm);
    return rc;
}
