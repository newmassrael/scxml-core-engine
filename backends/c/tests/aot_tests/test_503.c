// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test503 — C11 AOT runner.
//
// W3C SCXML 3.13: targetless transition runs its body assign without
// exiting/re-entering the source state
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST503_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test503_sm.h"

int main(void) {
    test503_t sm;
    REQUIRE_STARTED(test503_init(&sm), TEST503_START_OK);
    test503_run(&sm);

    int rc = test503_ended_in(&sm, TEST503_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test503: FAIL — active = 0x%08x\n", (unsigned)test503_active_states(&sm));
    }
    test503_destroy(&sm);
    return rc;
}
