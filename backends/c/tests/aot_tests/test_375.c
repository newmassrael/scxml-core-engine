// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test375 — C11 AOT runner.
//
// W3C SCXML 3.13: multi-block onentry handlers execute in document order
// (event1 before event2)

#include <stdio.h>

#include "../require_started.h"
#include "test375_sm.h"

int main(void) {
    test375_t sm;
    REQUIRE_STARTED(test375_init(&sm), TEST375_START_OK);
    test375_run(&sm);

    int rc = test375_ended_in(&sm, TEST375_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test375: FAIL — active = 0x%08x\n", (unsigned)test375_active_states(&sm));
    }
    test375_destroy(&sm);
    return rc;
}
