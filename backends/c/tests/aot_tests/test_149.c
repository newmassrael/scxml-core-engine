// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test149 — C11 AOT runner.
//
// W3C SCXML 4.3: neither <if> nor <elseif> executes when both conditions are
// false
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST149_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test149_sm.h"

int main(void) {
    test149_t sm;
    REQUIRE_STARTED(test149_init(&sm), TEST149_START_OK);
    test149_run(&sm);

    int rc = test149_ended_in(&sm, TEST149_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test149: FAIL — active = 0x%08x\n", (unsigned)test149_active_states(&sm));
    }
    test149_destroy(&sm);
    return rc;
}
