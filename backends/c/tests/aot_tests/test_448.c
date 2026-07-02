// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test448 — C11 AOT runner.
//
// W3C SCXML B.2: ECMAScript datamodel keeps every `<data>` declaration
// in a single global scope, so a value set inside one state's
// `<assign>` is observable by another state's transition cond. The
// fixture sets Var1 in s0 then exits to s1, whose cond `Var1 == 1`
// must read the same binding. Lua's globals naturally provide single-
// scope semantics, so this fixture verifies the existing
// scriptengine.jinja2 init / lua_assign emit shape preserves it.

#include <stdio.h>

#include "test448_sm.h"

int main(void) {
    test448_t sm;
    test448_init(&sm);
    test448_run(&sm);

    int rc = test448_in_state(&sm, TEST448_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test448: FAIL — active = 0x%08x\n", (unsigned)test448_active_states(&sm));
    }
    test448_destroy(&sm);
    return rc;
}
