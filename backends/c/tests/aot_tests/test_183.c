// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test183 — C11 AOT runner.
//
// W3C SCXML 6.2.4: `<send event="event1" idlocation="Var1"/>` stores the
// auto-generated sendid into the lua datamodel variable Var1 just before
// dispatching the bare-external event. The receiving transition's cond
// `typeof Var1 !== 'undefined'` reads Var1 (now bound to the `__send_N`
// string) and routes to pass.

#include <stdio.h>

#include "test183_sm.h"

int main(void) {
    test183_t sm;
    test183_init(&sm);
    test183_run(&sm);

    int rc = test183_in_state(&sm, TEST183_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test183: FAIL — active = 0x%08x\n", (unsigned)test183_active_states(&sm));
    }
    test183_destroy(&sm);
    return rc;
}
