// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test405 — C11 AOT runner.
//
// W3C SCXML 3.13: multi-leaf simultaneous fire — two region-internal
// eventless transitions in one microstep produce queue [event1, event2,
// event3, event4] via App.D.2 reverse-doc-order exits then doc-order bodies
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST405_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test405_sm.h"

int main(void) {
    test405_t sm;
    REQUIRE_STARTED(test405_init(&sm), TEST405_START_OK);
    test405_run(&sm);

    int rc = test405_ended_in(&sm, TEST405_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test405: FAIL — active = 0x%08x\n", (unsigned)test405_active_states(&sm));
    }
    test405_destroy(&sm);
    return rc;
}
