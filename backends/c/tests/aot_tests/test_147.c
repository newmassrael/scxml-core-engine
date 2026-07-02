// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test147 — C11 AOT runner.
//
// W3C SCXML 4.3 / 5.4: only the first <if>/<elseif>/<else> clause whose
// cond evaluates to true executes its body, and that body's <assign>
// updates the datamodel. test147's onentry fires `if (false)` (skip) /
// `elseif (true)` (raise bar + Var1=Var1+1) / falls past <else>. After
// the <if> block it raises bat. The transition `event="bar" cond="Var1
// == 1"` then fires into pass; the `event="*"` fallback into fail must
// NOT fire (its only role is to catch a buggy engine that picked the
// wrong branch).
//
// The Lua datamodel is owned by the SM struct; _destroy releases the
// lua_State. Calling _destroy is contract-mandatory whenever _init has
// been called (idempotent / NULL-safe inside).

#include <stdio.h>

#include "test147_sm.h"

int main(void) {
    test147_t sm;
    test147_init(&sm);
    test147_run(&sm);

    int rc = test147_in_state(&sm, TEST147_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test147: FAIL — active = 0x%08x\n", (unsigned)test147_active_states(&sm));
    }
    test147_destroy(&sm);
    return rc;
}
