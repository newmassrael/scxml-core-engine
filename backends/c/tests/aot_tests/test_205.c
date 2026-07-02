// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test205 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: a bare-external `<send event="event1"><param/></send>`
// followed by a bare-external `<send event="timeout"/>` (no params) puts
// two events on the external queue in doc order. mainEventLoop dequeues
// event1 first, set_current_event promotes `_pending_donedata = {aParam=1}`
// onto `_event.data`, the matching transition assigns Var1 = _event.data.aParam
// (= 1), then the slot is reset to nil so the subsequent timeout dequeue
// sees `_event.data = nil`. s1's cond `Var1 == 1` matches pass.

#include <stdio.h>

#include "test205_sm.h"

int main(void) {
    test205_t sm;
    test205_init(&sm);
    test205_run(&sm);

    int rc = test205_in_state(&sm, TEST205_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test205: FAIL — active = 0x%08x\n", (unsigned)test205_active_states(&sm));
    }
    test205_destroy(&sm);
    return rc;
}
