// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test561 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(561) invocation.

#include <stdio.h>

#include "test561_sm.h"

int main(void) {
    test561_t sm;
    test561_init(&sm);
    test561_run(&sm);

    int rc = test561_in_state(&sm, TEST561_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test561: FAIL — active = 0x%08x\n", (unsigned)test561_active_states(&sm));
    }
    test561_destroy(&sm);
    return rc;
}
