// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test326 — C11 AOT runner.
//
// W3C SCXML 5.10: assignment to _ioprocessors raises error.execution and
// leaves the binding unchanged
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST326_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test326_sm.h"

int main(void) {
    test326_t sm;
    REQUIRE_STARTED(test326_init(&sm), TEST326_START_OK);
    test326_run(&sm);

    int rc = test326_ended_in(&sm, TEST326_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test326: FAIL — active = 0x%08x\n", (unsigned)test326_active_states(&sm));
    }
    test326_destroy(&sm);
    return rc;
}
