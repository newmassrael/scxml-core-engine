// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test173 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send targetexpr="...">` evaluates the target string at
// send time, not at parse time. The fixture initialises Var1 to an
// invalid session id, then reassigns Var1 = '#_internal' in onentry just
// before the send. The runtime targetexpr eval reads the current Var1
// ('#_internal'), routes the event to the internal queue, and the
// receiving transition matches event1 → pass. If the eval used the initial
// value the dispatch would raise error.execution and the wildcard would
// route to fail.
//
// W3C SCXML 6.2: <send targetexpr> evaluates Var1 at send time (Var1
// reassigned to '#_internal' before send) — runtime target buffer drives the
// codegen-time `#_internal` / `!`-prefix / unreachable dispatch arms; cpp
// `SendHelper::isInternalTarget`/`isInvalidTarget` are inlined as
// stack-buffer prefix checks (jinja2-inline contract)

#include <stdio.h>

#include "../require_started.h"
#include "test173_sm.h"

int main(void) {
    test173_t sm;
    REQUIRE_STARTED(test173_init(&sm), TEST173_START_OK);
    test173_run(&sm);

    int rc = test173_ended_in(&sm, TEST173_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test173: FAIL — active = 0x%08x\n", (unsigned)test173_active_states(&sm));
    }
    test173_destroy(&sm);
    return rc;
}
