// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test294 — C11 AOT runner.
//
// W3C SCXML 5.5: nested compound finals — s02 builds `_sce_donedata =
// {Var1=1}` from <param expr="1"/> so cond `_event.data.Var1 == 1` matches;
// s12 (sibling) ships an inline `'foo'` content expression so the next
// done.state pass shares the same lua promotion path
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST294_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test294_sm.h"

int main(void) {
    test294_t sm;
    REQUIRE_STARTED(test294_init(&sm), TEST294_START_OK);
    test294_run(&sm);

    int rc = test294_ended_in(&sm, TEST294_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test294: FAIL — active = 0x%08x\n", (unsigned)test294_active_states(&sm));
    }
    test294_destroy(&sm);
    return rc;
}
