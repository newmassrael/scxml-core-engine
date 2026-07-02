// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test313 — C11 AOT runner.
//
// W3C SCXML 5.4: `<assign>` to an undeclared variable raises `error.execution` — the existing scriptengine.jinja2
// lua_assign macro's rc-checked luaL_dostring (8b1bb1e9) catches the chunk failure and enqueues the platform error,
// which the receiving transition matches before the wildcard fallback fires.

#include <stdio.h>

#include "test313_sm.h"

int main(void) {
    test313_t sm;
    test313_init(&sm);
    test313_run(&sm);

    int rc = test313_in_state(&sm, TEST313_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test313: FAIL — active = 0x%08x\n", (unsigned)test313_active_states(&sm));
    }
    test313_destroy(&sm);
    return rc;
}
