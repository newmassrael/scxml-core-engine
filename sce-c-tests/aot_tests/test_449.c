// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test449 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(449) invocation.

#include <stdio.h>

#include "test449_sm.h"

int main(void) {
    test449_t sm;
    test449_init(&sm);
    test449_run(&sm);

    test449_state_t final = test449_get_current_state(&sm);
    int rc = (final == TEST449_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test449: FAIL — final state = %d\n", (int)final);
    }
    test449_destroy(&sm);
    return rc;
}
