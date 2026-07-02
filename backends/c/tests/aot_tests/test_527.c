// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test527 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(527) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test527_sm.h"

int main(void) {
    test527_t sm;
    test527_init(&sm);
    test527_run(&sm);

    int rc = test527_in_state(&sm, TEST527_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test527: FAIL — active = 0x%08x\n", (unsigned)test527_active_states(&sm));
    }
    test527_destroy(&sm);
    return rc;
}
