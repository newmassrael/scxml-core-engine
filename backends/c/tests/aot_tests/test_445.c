// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test445 — C11 AOT runner.
//
// W3C SCXML 5.2: <data id> without expr/content is the ECMAScript value
// undefined (lua nil after transpile)

#include <stdio.h>

#include "test445_sm.h"

int main(void) {
    test445_t sm;
    test445_init(&sm);
    test445_run(&sm);

    int rc = test445_ended_in(&sm, TEST445_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test445: FAIL — active = 0x%08x\n", (unsigned)test445_active_states(&sm));
    }
    test445_destroy(&sm);
    return rc;
}
