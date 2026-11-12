// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test321 — C11 AOT runner.
//
// W3C SCXML 5.10: _sessionid is bound on startup (typeof Var1 !== 'undefined'
// after Var1 = _sessionid)
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST321_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test321_sm.h"

int main(void) {
    test321_t sm;
    REQUIRE_STARTED(test321_init(&sm), TEST321_START_OK);
    test321_run(&sm);

    int rc = test321_ended_in(&sm, TEST321_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test321: FAIL — active = 0x%08x\n", (unsigned)test321_active_states(&sm));
    }
    test321_destroy(&sm);
    return rc;
}
