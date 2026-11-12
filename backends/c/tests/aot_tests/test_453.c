// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test453 — C11 AOT runner.
//
// W3C SCXML B.2 ECMAScript function-expression in value-expression context
// (test453): `<data id="var1" expr="function(invar) {return invar + 1;}"/>`
// previously lowered to `var1 = (function(invar) {return invar + 1\n})` — the
// trailing `}` was left raw, which is invalid Lua. transform_expression now
// invokes transform_function_syntax (mirroring the script pipeline's order at
// lua_transformer.rs:1822), so the same anonymous-function lowering that
// already worked inside `<script>` blocks now applies wherever an ECMAScript
// value-expression is accepted (cond, location/data/assign expr). Result:
// `var1 = (function(invar)  return invar + 1\n end)`, then `var1(2)` invokes
// natively and the cond `var1(2) == 3` matches.
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST453_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test453_sm.h"

int main(void) {
    test453_t sm;
    REQUIRE_STARTED(test453_init(&sm), TEST453_START_OK);
    test453_run(&sm);

    int rc = test453_ended_in(&sm, TEST453_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test453: FAIL — active = 0x%08x\n", (unsigned)test453_active_states(&sm));
    }
    test453_destroy(&sm);
    return rc;
}
