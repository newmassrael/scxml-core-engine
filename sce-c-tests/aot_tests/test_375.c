// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test375 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(375) invocation.

#include <stdio.h>

#include "test375_sm.h"

int main(void) {
    test375_t sm;
    test375_init(&sm);
    test375_run(&sm);

    test375_state_t final = test375_get_current_state(&sm);
    int rc = (final == TEST375_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test375: FAIL — final state = %d\n", (int)final);
    }
    test375_destroy(&sm);
    return rc;
}
