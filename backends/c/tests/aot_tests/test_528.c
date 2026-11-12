// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test528 — C11 AOT runner.
//
// W3C SCXML 5.5: <content expr="undefined.invalidProperty"/> raises a runtime
// lua error — the chunk failure path empties `_sce_donedata` (W3C 5.5 "empty
// data on evaluation failure"), queues error.execution, and continues with
// the done.state.s0 raise; the s0 transition to s1 matches error.execution
// first, then s1 receives done.state.s0 with empty data and routes to pass
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST528_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test528_sm.h"

int main(void) {
    test528_t sm;
    REQUIRE_STARTED(test528_init(&sm), TEST528_START_OK);
    test528_run(&sm);

    int rc = test528_ended_in(&sm, TEST528_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test528: FAIL — active = 0x%08x\n", (unsigned)test528_active_states(&sm));
    }
    test528_destroy(&sm);
    return rc;
}
