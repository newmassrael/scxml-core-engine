// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test550 — C11 AOT runner.
//
// W3C SCXML 5.3: explicit binding="early" pre-initialises state-local <data
// expr> from a sibling state's transition cond

#include <stdio.h>

#include "../require_started.h"
#include "test550_sm.h"

int main(void) {
    test550_t sm;
    REQUIRE_STARTED(test550_init(&sm), TEST550_START_OK);
    test550_run(&sm);

    int rc = test550_ended_in(&sm, TEST550_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test550: FAIL — active = 0x%08x\n", (unsigned)test550_active_states(&sm));
    }
    test550_destroy(&sm);
    return rc;
}
