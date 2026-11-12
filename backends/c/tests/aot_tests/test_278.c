// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test278 — C11 AOT runner.
//
// W3C SCXML 5.3: state-local <datamodel> is reachable from outside its
// lexical scope (default early binding)

#include <stdio.h>

#include "../require_started.h"
#include "test278_sm.h"

int main(void) {
    test278_t sm;
    REQUIRE_STARTED(test278_init(&sm), TEST278_START_OK);
    test278_run(&sm);

    int rc = test278_ended_in(&sm, TEST278_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test278: FAIL — active = 0x%08x\n", (unsigned)test278_active_states(&sm));
    }
    test278_destroy(&sm);
    return rc;
}
