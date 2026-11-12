// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test322 — C11 AOT runner.
//
// W3C SCXML 5.10: assignment to _sessionid raises error.execution and leaves
// the binding unchanged
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST322_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test322_sm.h"

int main(void) {
    test322_t sm;
    REQUIRE_STARTED(test322_init(&sm), TEST322_START_OK);
    test322_run(&sm);

    int rc = test322_ended_in(&sm, TEST322_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test322: FAIL — active = 0x%08x\n", (unsigned)test322_active_states(&sm));
    }
    test322_destroy(&sm);
    return rc;
}
