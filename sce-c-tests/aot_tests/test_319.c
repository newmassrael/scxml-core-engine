// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test319 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(319) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test319_sm.h"

int main(void) {
    test319_t sm;
    test319_init(&sm);
    test319_run(&sm);

    test319_state_t final = test319_get_current_state(&sm);
    int rc = (final == TEST319_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test319: FAIL — final state = %d\n", (int)final);
    }
    test319_destroy(&sm);
    return rc;
}
