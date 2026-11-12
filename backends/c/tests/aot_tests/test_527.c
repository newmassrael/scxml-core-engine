// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test527 — C11 AOT runner.
//
// W3C SCXML 5.5: <donedata><content expr="'foo'"/> evaluates the expression
// at final-state entry and the resulting string rides on done.state's
// `data[]` to `_event.data` so the cond `_event.data == 'foo'` matches the
// pass transition
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST527_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test527_sm.h"

int main(void) {
    test527_t sm;
    REQUIRE_STARTED(test527_init(&sm), TEST527_START_OK);
    test527_run(&sm);

    int rc = test527_ended_in(&sm, TEST527_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test527: FAIL — active = 0x%08x\n", (unsigned)test527_active_states(&sm));
    }
    test527_destroy(&sm);
    return rc;
}
