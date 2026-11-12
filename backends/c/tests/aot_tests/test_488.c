// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test488 — C11 AOT runner.
//
// W3C SCXML 5.7: <param expr="undefined.invalidProperty"/> — base identifier
// `undefined` is undeclared so the chunk raises ReferenceError, the macro
// maps it to error.execution, and the runner's continue-with-empty-data path
// emits `done.state.s0` with no payload (s1 then matches it to pass)
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST488_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test488_sm.h"

int main(void) {
    test488_t sm;
    REQUIRE_STARTED(test488_init(&sm), TEST488_START_OK);
    test488_run(&sm);

    int rc = test488_ended_in(&sm, TEST488_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test488: FAIL — active = 0x%08x\n", (unsigned)test488_active_states(&sm));
    }
    test488_destroy(&sm);
    return rc;
}
