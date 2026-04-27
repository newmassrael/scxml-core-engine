// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test277 — C11 AOT runner.
//
// W3C SCXML 5.3: When the value specified for a <data> (via 'expr',
// 'src', or inline content) is not a legal data value, the SCXML
// processor MUST raise error.execution AND leave the data element
// undefined so that a later <assign> can populate it. test277's
// top-level <data id="Var1" expr="undefined.invalidProperty"/>
// triggers exactly that path: the expression fails to evaluate, the
// queued error.execution drives s0's `transition event="error.execution"
// target="s1"`, and s1's onentry assigns Var1 = 1 to prove the
// location is still writable. The transition `cond="Var1 == 1"` then
// reaches `pass`.
//
// First consumer of the lua_init_engine var-init failure path: prior
// fixtures only exercised the silent-drop policy (test287 — legal
// assign, test445 — `<data id>` without expr, test309 — non-boolean
// cond). The rc-checked emit shape introduced for this family applies
// to the top-level datamodel loop in scriptengine.jinja2's
// lua_init_engine. State-local datamodel init failure remains
// silent-drop (carve-out — no current fixture exercises it).
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(277) invocation.

#include <stdio.h>

#include "test277_sm.h"

int main(void) {
    test277_t sm;
    test277_init(&sm);
    test277_run(&sm);

    int rc = test277_in_state(&sm, TEST277_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test277: FAIL — active = 0x%08x\n", (unsigned)test277_active_states(&sm));
    }
    test277_destroy(&sm);
    return rc;
}
