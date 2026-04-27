// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test176 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: `<send>` evaluates `<param>` expressions at send
// time, not at delivery time. The fixture starts with Var1 = 1 then
// reassigns Var1 = 2 in onentry before the send, so `<param expr="Var1"/>`
// must capture 2. The receiving transition assigns Var2 = _event.data.aParam
// (= 2), and the next state's cond `Var2 == 2` matches the pass transition.
// The send is bare-external (no target), so `_pending_donedata` is
// promoted onto `_event.data` by the next dequeue's `set_current_event`.

#include <stdio.h>

#include "test176_sm.h"

int main(void) {
    test176_t sm;
    test176_init(&sm);
    test176_run(&sm);

    int rc = test176_in_state(&sm, TEST176_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test176: FAIL — active = 0x%08x\n", (unsigned)test176_active_states(&sm));
    }
    test176_destroy(&sm);
    return rc;
}
