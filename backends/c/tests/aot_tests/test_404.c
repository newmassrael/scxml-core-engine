// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test404 — C11 AOT runner.
//
// W3C SCXML 3.4: <parallel> entry activates every region simultaneously; exit
// reverse-doc-order (s01p2 before s01p1 before s01p) drains queued raises in
// order so receiving states match event1→event2→event3→event4
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST404_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test404_sm.h"

int main(void) {
    test404_t sm;
    REQUIRE_STARTED(test404_init(&sm), TEST404_START_OK);
    test404_run(&sm);

    int rc = test404_ended_in(&sm, TEST404_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test404: FAIL — active = 0x%08x\n", (unsigned)test404_active_states(&sm));
    }
    test404_destroy(&sm);
    return rc;
}
