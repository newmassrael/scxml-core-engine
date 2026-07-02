// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test346 — C11 AOT runner.
//
// W3C SCXML 5.10: every illegal `<assign>` to a read-only system
// variable MUST raise `error.execution`. The fixture cycles through
// four states and four system vars (`_sessionid` → `_event` →
// `_ioprocessors` → `_name`), each one paired with a `<raise>` of a
// distinct dummy event. PASS requires that on each step the
// `error.execution` lands in the queue *before* its sibling dummy is
// consumed, so the `event="error.execution" target="..."` transition
// fires preemption-correctly while the dummy is dropped via a
// targetless transition. The addition of `_event` to the codegen-time
// reserved-name guard is what activates s1's `error.execution → s2`
// branch; without it the assign would silently succeed on the lua
// state and the wildcard fallback would route to fail.

#include <stdio.h>

#include "test346_sm.h"

int main(void) {
    test346_t sm;
    test346_init(&sm);
    test346_run(&sm);

    int rc = test346_in_state(&sm, TEST346_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test346: FAIL — active = 0x%08x\n", (unsigned)test346_active_states(&sm));
    }
    test346_destroy(&sm);
    return rc;
}
