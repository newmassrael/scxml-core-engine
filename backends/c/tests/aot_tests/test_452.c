// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test452 — C11 AOT runner.
//
// W3C SCXML B.2 ECMAScript constructor + dotted location (test452):
// transform_script's existing constructor pattern (FuncInfo.is_constructor
// when body contains `this.`) lifts `function testobject() { this.bar = 0; }`
// to `function testobject() local self = {} self.bar = 0 return self end`,
// then transform_new_expression rewrites `new testobject()` to `testobject()`
// so the bare call returns the table; transform_expression's existing
// dotted-location path lets `<assign location="foo.bar" expr="1"/>` lower to
// `foo.bar = (1)` without further changes, and the cond `foo.bar == 1` reads
// back natively. No template change needed — verifies the script-side
// constructor lift and the expression-side new/dotted-location combine
// without conflict.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST452_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test452_sm.h"

int main(void) {
    test452_t sm;
    REQUIRE_STARTED(test452_init(&sm), TEST452_START_OK);
    test452_run(&sm);

    int rc = test452_ended_in(&sm, TEST452_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test452: FAIL — active = 0x%08x\n", (unsigned)test452_active_states(&sm));
    }
    test452_destroy(&sm);
    return rc;
}
