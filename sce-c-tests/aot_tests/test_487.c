// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test487 — C11 AOT runner.
//
// W3C SCXML 5.4: Same illegal-assign path as test312 but the target
// data variable is declared without an `expr` (so it starts as the
// ECMAScript `undefined`, lua nil after transpile — see test445 for
// the no-expr declaration semantics). Assigning `undefined.invalidProperty`
// to it still fails the lua chunk and raises error.execution; the
// onentry's subsequent `<raise event="event"/>` lands behind it in
// the queue, and s0's `event="error.execution" target="pass"`
// transition fires first.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(487) invocation.

#include <stdio.h>

#include "test487_sm.h"

int main(void) {
    test487_t sm;
    test487_init(&sm);
    test487_run(&sm);

    int rc = test487_in_state(&sm, TEST487_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test487: FAIL — active = 0x%08x\n", (unsigned)test487_active_states(&sm));
    }
    test487_destroy(&sm);
    return rc;
}
