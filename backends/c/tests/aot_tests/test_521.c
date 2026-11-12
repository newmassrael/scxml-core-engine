// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test521 — C11 AOT runner.
//
// W3C SCXML 6.2: dispatching `<send target="#_session_<missing>">` against a non-existent session URI raises
// `error.communication` — the C11 targetexpr arm's unreachable-fallback clause classifies the literal as not-empty /
// not-#_internal / not-!-prefixed and falls through to the error-communication raise. Receiving transition matches and
// routes to pass. Same code path test496 already pins.
//
// W3C SCXML 6.2: <send targetexpr="undefined"/> evaluates to a
// non-dispatchable target — the targetexpr arm's unreachable-fallback clause
// raises error.communication (same arm test496 already covers).

#include <stdio.h>

#include "../require_started.h"
#include "test521_sm.h"

int main(void) {
    test521_t sm;
    REQUIRE_STARTED(test521_init(&sm), TEST521_START_OK);
    test521_run(&sm);

    int rc = test521_ended_in(&sm, TEST521_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test521: FAIL — active = 0x%08x\n", (unsigned)test521_active_states(&sm));
    }
    test521_destroy(&sm);
    return rc;
}
