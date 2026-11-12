// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test449 — C11 AOT runner.
//
// W3C SCXML 5.9: a non-empty string in transition cond is a truthy value
// (lua_toboolean returns true)

#include <stdio.h>

#include "../require_started.h"
#include "test449_sm.h"

int main(void) {
    test449_t sm;
    REQUIRE_STARTED(test449_init(&sm), TEST449_START_OK);
    test449_run(&sm);

    int rc = test449_ended_in(&sm, TEST449_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test449: FAIL — active = 0x%08x\n", (unsigned)test449_active_states(&sm));
    }
    test449_destroy(&sm);
    return rc;
}
