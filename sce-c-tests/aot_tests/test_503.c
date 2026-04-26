// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test503 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(503) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test503_sm.h"

int main(void) {
    test503_t sm;
    test503_init(&sm);
    test503_run(&sm);

    test503_state_t final = test503_get_current_state(&sm);
    int rc = (final == TEST503_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test503: FAIL — final state = %d\n", (int)final);
    }
    test503_destroy(&sm);
    return rc;
}
