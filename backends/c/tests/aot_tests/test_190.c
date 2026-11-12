// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//
// W3C SCXML 6.2 + C.1.5 + App.D.2: targetexpr-resolved session URI
// (`'#_scxml_'`) routes onto the external queue per cpp's fall-through to
// `engine.raiseExternal` after the invalid/unreachable/internal helper checks
// — the C11 mirror replaces the prior strict `else { error.communication;
// return; }` with a two-step (1) explicit `"undefined"`/`"nil"` unreachable
// arm and (2) raise_external default; the s0 onentry queues event2 (external
// via targetexpr), event1 (internal via raise), timeout (external bare), and
// the internal-first drain pops event1 → s0→s1, then external pop yields
// event2 → s1→pass before timeout's wildcard fail.

#include <stdio.h>

#include "../require_started.h"
#include "test190_sm.h"

int main(void) {
    test190_t sm;
    REQUIRE_STARTED(test190_init(&sm), TEST190_START_OK);
    test190_run(&sm);

    int rc = test190_ended_in(&sm, TEST190_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test190: FAIL — active = 0x%08x\n", (unsigned)test190_active_states(&sm));
    }
    test190_destroy(&sm);
    return rc;
}
