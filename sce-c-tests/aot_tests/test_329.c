// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test329 — C11 AOT runner.
//
// W3C SCXML 5.10: read-only system variables (`_sessionid`, `_event`,
// `_name`, `_ioprocessors`) cannot be modified — `<assign>` to any of
// them is silently ignored on the binding's value (the codegen-time
// reserved-name guard rejects the lua chunk and raises error.execution,
// leaving the prior binding in place). The fixture chains four states:
// each state captures the current binding into a Var, attempts an
// illegal `<assign>` with a poison value, then verifies the binding
// is unchanged via cond `Var{n} == _systemvar`. PASS requires every
// chain to hold — the addition of `_event` to the reserved list is
// what unblocks state s1's `Var2 == _event` (Var2 captured the foo
// table reference, _event must remain the same table).

#include <stdio.h>

#include "test329_sm.h"

int main(void) {
    test329_t sm;
    test329_init(&sm);
    test329_run(&sm);

    int rc = test329_in_state(&sm, TEST329_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test329: FAIL — active = 0x%08x\n", (unsigned)test329_active_states(&sm));
    }
    test329_destroy(&sm);
    return rc;
}
