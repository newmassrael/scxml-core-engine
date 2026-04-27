// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test286 — C11 AOT runner.
//
// W3C SCXML 5.4: An <assign> whose location attribute is empty (or
// resolves to a non-declared variable) MUST raise error.execution.
// test286 emits `<assign location="" expr="1"/>` followed by
// `<raise event="foo"/>` in s0's onentry; the empty location turns
// the lua chunk into ` = (1)` which fails to parse, and the rc-checked
// lua_assign macro queues error.execution before the foo raise reaches
// the queue. s0's first matching transition is then
// `event="error.execution" target="pass"`.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(286) invocation.

#include <stdio.h>

#include "test286_sm.h"

int main(void) {
    test286_t sm;
    test286_init(&sm);
    test286_run(&sm);

    int rc = test286_in_state(&sm, TEST286_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test286: FAIL — active = 0x%08x\n", (unsigned)test286_active_states(&sm));
    }
    test286_destroy(&sm);
    return rc;
}
