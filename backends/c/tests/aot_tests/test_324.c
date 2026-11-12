// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test324 — C11 AOT runner.
//
// W3C SCXML 5.10: _name remains bound through the session — assignment to
// _name does not change the binding
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST324_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test324_sm.h"

int main(void) {
    test324_t sm;
    REQUIRE_STARTED(test324_init(&sm), TEST324_START_OK);
    test324_run(&sm);

    int rc = test324_ended_in(&sm, TEST324_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test324: FAIL — active = 0x%08x\n", (unsigned)test324_active_states(&sm));
    }
    test324_destroy(&sm);
    return rc;
}
