// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test337 — C11 AOT runner.
//
// W3C SCXML 5.10.1: `_event.origintype` follows the same internal-vs-external binding rule as `_event.origin` —
// internal pops leave it nil, and the cond `typeof _event.origintype === 'undefined'` matches the pass transition.
// Direct sibling of test335 over the origintype field.

#include <stdio.h>

#include "test337_sm.h"

int main(void) {
    test337_t sm;
    test337_init(&sm);
    test337_run(&sm);

    int rc = test337_in_state(&sm, TEST337_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test337: FAIL — active = 0x%08x\n", (unsigned)test337_active_states(&sm));
    }
    test337_destroy(&sm);
    return rc;
}
