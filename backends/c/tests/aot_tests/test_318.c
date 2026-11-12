// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test318 — C11 AOT runner.
//
// W3C SCXML 5.10: _event.name stays bound across onexit and entry into the
// next state
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST318_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test318_sm.h"

int main(void) {
    test318_t sm;
    REQUIRE_STARTED(test318_init(&sm), TEST318_START_OK);
    test318_run(&sm);

    int rc = test318_ended_in(&sm, TEST318_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test318: FAIL — active = 0x%08x\n", (unsigned)test318_active_states(&sm));
    }
    test318_destroy(&sm);
    return rc;
}
