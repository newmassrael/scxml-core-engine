// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test312 — C11 AOT runner.
//
// W3C SCXML 5.4: An <assign> whose `expr` evaluates to an illegal
// value (here `undefined.invalidProperty`) MUST raise error.execution.
// s0's onentry runs the failing assign before its `<raise event="foo"/>`,
// so the queue order is [error.execution, foo] and s0's first matching
// transition `event="error.execution" target="pass"` fires before the
// `event=".*" target="fail"` trap is even considered (first-match-wins
// per W3C 3.13). The `.*` form here exercises the C11 wildcard
// synonym path — `*`, `.*`, `_*` collapse to "match any non-NONE
// event" in line with analyzer.rs::collect_implicit_event_descriptors.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(312) invocation.

#include <stdio.h>

#include "test312_sm.h"

int main(void) {
    test312_t sm;
    test312_init(&sm);
    test312_run(&sm);

    test312_state_t final = test312_get_current_state(&sm);
    int rc = (final == TEST312_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test312: FAIL — final state = %d\n", (int)final);
    }
    test312_destroy(&sm);
    return rc;
}
