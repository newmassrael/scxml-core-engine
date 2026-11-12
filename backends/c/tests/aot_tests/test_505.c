// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test505 — C11 AOT runner.
//
// W3C SCXML 3.13: internal transition (target is proper descendant of source)
// runs body without exiting source state
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST505_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test505_sm.h"

int main(void) {
    test505_t sm;
    REQUIRE_STARTED(test505_init(&sm), TEST505_START_OK);
    test505_run(&sm);

    int rc = test505_ended_in(&sm, TEST505_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test505: FAIL — active = 0x%08x\n", (unsigned)test505_active_states(&sm));
    }
    test505_destroy(&sm);
    return rc;
}
