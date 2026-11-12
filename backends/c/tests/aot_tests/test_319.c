// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test319 — C11 AOT runner.
//
// W3C SCXML 5.10: _event is unbound (lua nil) until the first event has been
// dequeued
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST319_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test319_sm.h"

int main(void) {
    test319_t sm;
    REQUIRE_STARTED(test319_init(&sm), TEST319_START_OK);
    test319_run(&sm);

    int rc = test319_ended_in(&sm, TEST319_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test319: FAIL — active = 0x%08x\n", (unsigned)test319_active_states(&sm));
    }
    test319_destroy(&sm);
    return rc;
}
