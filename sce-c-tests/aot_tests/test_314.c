// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test314 — C11 AOT runner.
//
// W3C SCXML 5.4: `<assign expr=...>` whose expression evaluates to undefined / null / non-existent property raises `error.execution` — same lua_assign error-path that test313 covers, exercised through a different illegal-expression shape (right-hand-side resolution failure rather than illegal location).

#include <stdio.h>

#include "test314_sm.h"

int main(void) {
    test314_t sm;
    test314_init(&sm);
    test314_run(&sm);

    int rc = test314_in_state(&sm, TEST314_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test314: FAIL — active = 0x%08x\n", (unsigned)test314_active_states(&sm));
    }
    test314_destroy(&sm);
    return rc;
}
