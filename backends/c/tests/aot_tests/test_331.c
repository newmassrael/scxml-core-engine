// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test331 — C11 AOT runner.
//
// W3C SCXML 5.10.1: `_event.type` carries the three-way spec
// category ('platform' / 'internal' / 'external') so receiver
// state cond expressions can branch on event provenance. The
// fixture walks every category in document order: s0 onentry
// `<raise event="foo"/>` (internal queue), s2 onentry empty-
// location `<assign>` (the SCXML processor itself raises
// error.execution → 'platform'), and s4 onentry `<send event="foo"/>`
// (external queue). Each receiving transition asserts the
// expected category via `Var1 == '<category>'`. Without the
// `_pending_event_type` carry the cond chain falls through to
// the wildcard fail. With error.foo/done.foo families overriding
// to 'platform' inside set_current_event regardless of which
// queue popped them, the s2 dequeue of the internally-raised
// error.execution still binds Var1='platform' rather than
// 'internal'.

#include <stdio.h>

#include "test331_sm.h"

int main(void) {
    test331_t sm;
    test331_init(&sm);
    test331_run(&sm);

    int rc = test331_in_state(&sm, TEST331_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test331: FAIL — active = 0x%08x\n", (unsigned)test331_active_states(&sm));
    }
    test331_destroy(&sm);
    return rc;
}
