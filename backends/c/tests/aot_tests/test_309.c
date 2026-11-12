// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test309 — C11 AOT runner.
//
// W3C SCXML 5.9: a non-boolean transition cond evaluates to false
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST309_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test309_sm.h"

int main(void) {
    test309_t sm;
    REQUIRE_STARTED(test309_init(&sm), TEST309_START_OK);
    test309_run(&sm);

    int rc = test309_ended_in(&sm, TEST309_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test309: FAIL — active = 0x%08x\n", (unsigned)test309_active_states(&sm));
    }
    test309_destroy(&sm);
    return rc;
}
