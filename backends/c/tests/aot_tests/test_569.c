// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test569 — C11 AOT runner.
//
// W3C SCXML 5.10 + C.1 (ECMAScript datamodel): direct cond access of
// `_ioprocessors['scxml'].location` must evaluate truthy. With the
// lua_init_engine baseline `_ioprocessors = {scxml = {location =
// 'scxml'}}` (mirror of cpp `LuaEngine::setupSystemVariables`), the
// indexed access yields the literal string `"scxml"`, _scxml_truthy
// classifies a non-empty string as truthy, and s0→pass dispatches.

#include <stdio.h>

#include "test569_sm.h"

int main(void) {
    test569_t sm;
    test569_init(&sm);
    test569_run(&sm);

    int rc = test569_in_state(&sm, TEST569_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test569: FAIL — active = 0x%08x\n", (unsigned)test569_active_states(&sm));
    }
    test569_destroy(&sm);
    return rc;
}
