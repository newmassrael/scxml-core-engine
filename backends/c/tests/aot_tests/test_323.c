// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test323 — C11 AOT runner.
//
// W3C SCXML 5.10: _name is bound on startup (datamodel <data expr="_name">
// followed by typeof check)
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST323_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test323_sm.h"

int main(void) {
    test323_t sm;
    REQUIRE_STARTED(test323_init(&sm), TEST323_START_OK);
    test323_run(&sm);

    int rc = test323_ended_in(&sm, TEST323_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test323: FAIL — active = 0x%08x\n", (unsigned)test323_active_states(&sm));
    }
    test323_destroy(&sm);
    return rc;
}
