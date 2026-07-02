// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test200 — C11 AOT runner.
//
// W3C SCXML 6.2: `type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"` literal is supported and routes through
// bare-external dispatch — same send_type carve-out test348/495 cover (옵션 ρ); the receiving transition matches event1
// before the wildcard fail.

#include <stdio.h>

#include "test200_sm.h"

int main(void) {
    test200_t sm;
    test200_init(&sm);
    test200_run(&sm);

    int rc = test200_in_state(&sm, TEST200_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test200: FAIL — active = 0x%08x\n", (unsigned)test200_active_states(&sm));
    }
    test200_destroy(&sm);
    return rc;
}
