// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test332 — C11 AOT runner.
//
// W3C SCXML 5.10.1 + 6.2.4: `<send target="!invalid" event="foo" idlocation="Var1"/>`
// stores the auto-generated sendid into Var1, then the invalid target
// raises `error.execution` carrying the same sendid in the event
// metadata. The error transition assigns Var2 = `_event.sendid`, and
// the next state's cond `Var1 === Var2` confirms both values match —
// proving sendid round-trips through the codegen-time idlocation store
// and the runtime `_event.sendid` reader path (set_current_event's lua
// chunk reads send_id from the event_with_meta_t struct).

#include <stdio.h>

#include "test332_sm.h"

int main(void) {
    test332_t sm;
    test332_init(&sm);
    test332_run(&sm);

    int rc = test332_in_state(&sm, TEST332_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test332: FAIL — active = 0x%08x\n", (unsigned)test332_active_states(&sm));
    }
    test332_destroy(&sm);
    return rc;
}
