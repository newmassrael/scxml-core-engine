// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test279 — C11 AOT runner.
//
// W3C SCXML 5.3: early binding (default) initialises state-local <data expr>
// at startup before its owning state is entered

#include <stdio.h>

#include "test279_sm.h"

int main(void) {
    test279_t sm;
    test279_init(&sm);
    test279_run(&sm);

    int rc = test279_ended_in(&sm, TEST279_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test279: FAIL — active = 0x%08x\n", (unsigned)test279_active_states(&sm));
    }
    test279_destroy(&sm);
    return rc;
}
