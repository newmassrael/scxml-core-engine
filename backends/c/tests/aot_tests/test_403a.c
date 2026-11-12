// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test403a — C11 AOT runner.
//
// W3C SCXML 3.13: optimal enabled transition set — child transitions take
// precedence over parent matches; cond="false" literals lower to native C (no
// lua link)
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST403A_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test403a_sm.h"

int main(void) {
    test403a_t sm;
    REQUIRE_STARTED(test403a_init(&sm), TEST403A_START_OK);
    test403a_run(&sm);

    int rc = test403a_ended_in(&sm, TEST403A_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test403a: FAIL — active = 0x%08x\n", (unsigned)test403a_active_states(&sm));
    }
    test403a_destroy(&sm);
    return rc;
}
