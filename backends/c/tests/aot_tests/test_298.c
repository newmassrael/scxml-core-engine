// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test298 — C11 AOT runner.
//
// W3C SCXML 5.7: <param location=""/> names no location — error.execution,
// and the pair is ignored; error.execution is queued ahead of done.state.s0,
// so its transition reaches pass first, and the surrounding `<send
// event=timeout delay=1s>` schedule is unused because that macrostep reaches
// pass before the timer fires
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST298_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test298_sm.h"

int main(void) {
    test298_t sm;
    REQUIRE_STARTED(test298_init(&sm), TEST298_START_OK);
    test298_run(&sm);

    int rc = test298_ended_in(&sm, TEST298_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test298: FAIL — active = 0x%08x\n", (unsigned)test298_active_states(&sm));
    }
    test298_destroy(&sm);
    return rc;
}
