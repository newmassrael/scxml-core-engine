// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test521 — C11 AOT runner.
//
// W3C SCXML 6.2: dispatching `<send target="#_session_<missing>">` against a non-existent session URI raises `error.communication` — the C11 targetexpr arm's unreachable-fallback clause classifies the literal as not-empty / not-#_internal / not-!-prefixed and falls through to the error-communication raise. Receiving transition matches and routes to pass. Same code path test496 already pins.

#include <stdio.h>

#include "test521_sm.h"

int main(void) {
    test521_t sm;
    test521_init(&sm);
    test521_run(&sm);

    int rc = test521_in_state(&sm, TEST521_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test521: FAIL — active = 0x%08x\n", (unsigned)test521_active_states(&sm));
    }
    test521_destroy(&sm);
    return rc;
}
