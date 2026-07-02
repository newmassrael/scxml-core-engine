// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test518 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(518) invocation.

#include <stdio.h>

#include "test518_sm.h"

int main(void) {
    test518_t sm;
    test518_init(&sm);
    test518_run(&sm);

    int rc = test518_in_state(&sm, TEST518_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test518: FAIL — active = 0x%08x\n", (unsigned)test518_active_states(&sm));
    }
    test518_destroy(&sm);
    return rc;
}
