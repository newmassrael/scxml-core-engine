// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test325 — C11 AOT runner.
//
// W3C SCXML 5.10: _ioprocessors is bound at startup (datamodel <data
// expr="_ioprocessors"> followed by typeof check)
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST325_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test325_sm.h"

int main(void) {
    test325_t sm;
    REQUIRE_STARTED(test325_init(&sm), TEST325_START_OK);
    test325_run(&sm);

    int rc = test325_ended_in(&sm, TEST325_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test325: FAIL — active = 0x%08x\n", (unsigned)test325_active_states(&sm));
    }
    test325_destroy(&sm);
    return rc;
}
