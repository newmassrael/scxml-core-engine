// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test551 — C11 AOT runner.
//
// W3C SCXML 5.3: early binding initialises state-local <data> with inline
// content (transpiled to a Lua table literal)

#include <stdio.h>

#include "../require_started.h"
#include "test551_sm.h"

int main(void) {
    test551_t sm;
    REQUIRE_STARTED(test551_init(&sm), TEST551_START_OK);
    test551_run(&sm);

    int rc = test551_ended_in(&sm, TEST551_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test551: FAIL — active = 0x%08x\n", (unsigned)test551_active_states(&sm));
    }
    test551_destroy(&sm);
    return rc;
}
