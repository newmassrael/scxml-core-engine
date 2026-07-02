// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test150 — C11 AOT runner.
//
// W3C SCXML 4.6: <foreach> with empty body declares its item / index
// variables on the datamodel even when the array is non-empty (the
// declaration is part of the loop's contract, not a side effect of the
// body). test150 relies on this in two ways: s0 reuses already-declared
// vars (Var1, Var2), s1 introduces fresh vars (Var4, Var5) that did not
// exist on the datamodel before. The s2 pass-guard `typeof Var4 !==
// 'undefined'` exercises the second case.
//
// The transition `event="*"` in s0/s1 is also load-bearing: foo (raised
// after the foreach in s0) and bar (raised in s1) are matched by the
// wildcard, not by a typed transition. Document order plus
// first-match-wins means the prior `event="error"` transition is
// declined (no error is raised), and the wildcard then catches foo→s1
// and bar→s2.
//
// The Lua datamodel is owned by the SM struct; _destroy releases the
// lua_State.

#include <stdio.h>

#include "test150_sm.h"

int main(void) {
    test150_t sm;
    test150_init(&sm);
    test150_run(&sm);

    int rc = test150_in_state(&sm, TEST150_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test150: FAIL — active = 0x%08x\n", (unsigned)test150_active_states(&sm));
    }
    test150_destroy(&sm);
    return rc;
}
