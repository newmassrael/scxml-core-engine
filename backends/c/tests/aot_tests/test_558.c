// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test558 — C11 AOT runner.
//
// W3C SCXML B.2: inline `<data id="var1">\nthis  is \na string \n</data>`
// first attempts `var1 = (this  is \na string)` — Lua rejects this
// (unexpected identifier `is` after `this`), the new fallback path pushes the
// `normalize_ws`-filtered literal `"this is a string"` and `lua_setglobal`s
// it. The src= sibling reads `test558.txt` (same raw text) at codegen time
// and goes through the identical emit. cond `var == 'this is a string'`
// matches both vars → s0→s1→pass. Mirrors cpp
// `DataModelInitHelper::initializeVariable`'s eval-then-`normalizeWhitespace`
// dispatch (sce/src/common/DataModelInitHelper.cpp:121-156).

#include <stdio.h>

#include "../require_started.h"
#include "test558_sm.h"

int main(void) {
    test558_t sm;
    REQUIRE_STARTED(test558_init(&sm), TEST558_START_OK);
    test558_run(&sm);

    int rc = test558_ended_in(&sm, TEST558_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test558: FAIL — active = 0x%08x\n", (unsigned)test558_active_states(&sm));
    }
    test558_destroy(&sm);
    return rc;
}
