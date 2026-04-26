// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test279 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(279) invocation.

#include <stdio.h>

#include "test279_sm.h"

int main(void) {
    test279_t sm;
    test279_init(&sm);
    test279_run(&sm);

    test279_state_t final = test279_get_current_state(&sm);
    int rc = (final == TEST279_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test279: FAIL — final state = %d\n", (int)final);
    }
    test279_destroy(&sm);
    return rc;
}
