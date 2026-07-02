// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test501 — C11 AOT runner.
//
// W3C SCXML 5.10 + C.1: same `_ioprocessors['scxml']['location']` lift
// as test500 — once the datamodel `<data Var1
// expr="_ioprocessors['scxml']['location']"/>` initialises without
// raising error.execution, the s0 onentry's bare `<send event=foo/>`
// reaches the external queue and the foo-matching transition routes to
// pass. The 2 s safety-net timeout never fires because the SM halts in
// the pass final state on the first external pop.

#include <stdio.h>

#include "test501_sm.h"

int main(void) {
    test501_t sm;
    test501_init(&sm);
    test501_run(&sm);

    int rc = test501_in_state(&sm, TEST501_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test501: FAIL — active = 0x%08x\n", (unsigned)test501_active_states(&sm));
    }
    test501_destroy(&sm);
    return rc;
}
