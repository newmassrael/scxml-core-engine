// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test324 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(324) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test324_sm.h"

int main(void) {
    test324_t sm;
    test324_init(&sm);
    test324_run(&sm);

    test324_state_t final = test324_get_current_state(&sm);
    int rc = (final == TEST324_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test324: FAIL — final state = %d\n", (int)final);
    }
    test324_destroy(&sm);
    return rc;
}
