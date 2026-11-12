// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test529 — C11 AOT runner.
//
// W3C SCXML 5.5 + Appendix B.2.6: inline-text <donedata><content>21</content>
// with the ECMAScript datamodel is evaluated as an expression (parser
// collapses to DoneDataContent::Expression) — the numeric 21 round-trips
// through `_event.data` for the cond `_event.data == 21`
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST529_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test529_sm.h"

int main(void) {
    test529_t sm;
    REQUIRE_STARTED(test529_init(&sm), TEST529_START_OK);
    test529_run(&sm);

    int rc = test529_ended_in(&sm, TEST529_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test529: FAIL — active = 0x%08x\n", (unsigned)test529_active_states(&sm));
    }
    test529_destroy(&sm);
    return rc;
}
