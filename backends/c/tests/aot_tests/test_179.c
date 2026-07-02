// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test179 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2 + Appendix B.2: `<send>` with inline
// `<content>123</content>` carries the body as `_event.data`. With the
// ECMAScript datamodel, the parser collapses the literal text "123" into
// an expression value (numeric 123), so the receiving transition's cond
// `_event.data == 123` compares the lua number stashed on
// `_pending_donedata` to the JS number 123 and matches the pass branch.

#include <stdio.h>

#include "test179_sm.h"

int main(void) {
    test179_t sm;
    test179_init(&sm);
    test179_run(&sm);

    int rc = test179_in_state(&sm, TEST179_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test179: FAIL — active = 0x%08x\n", (unsigned)test179_active_states(&sm));
    }
    test179_destroy(&sm);
    return rc;
}
