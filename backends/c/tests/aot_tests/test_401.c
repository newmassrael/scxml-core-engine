// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test401 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: error events go on the internal queue while
// bare `<send>` lands on the external queue, and App.D.2's
// mainEventLoop drains internal first. test401's s0 onentry sends
// `foo` externally then assigns to an empty location (raises
// error.execution onto the internal queue). The internal-first drain
// must dispatch error.execution before `foo`, so the W3C 5.9.3
// dot-prefix matcher (descriptor `error` matches `error.execution`)
// routes to pass. A leak that pushes error.execution onto the external
// queue inverts the order and the runner ends in fail.

#include <stdio.h>

#include "test401_sm.h"

int main(void) {
    test401_t sm;
    test401_init(&sm);
    test401_run(&sm);

    int rc = test401_in_state(&sm, TEST401_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test401: FAIL — active = 0x%08x\n", (unsigned)test401_active_states(&sm));
    }
    test401_destroy(&sm);
    return rc;
}
