// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test148 — C11 AOT runner.
//
// W3C SCXML 4.3: <else> clause executes when <if>/<elseif> evaluate false
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST148_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test148_sm.h"

int main(void) {
    test148_t sm;
    REQUIRE_STARTED(test148_init(&sm), TEST148_START_OK);
    test148_run(&sm);

    int rc = test148_ended_in(&sm, TEST148_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test148: FAIL — active = 0x%08x\n", (unsigned)test148_active_states(&sm));
    }
    test148_destroy(&sm);
    return rc;
}
