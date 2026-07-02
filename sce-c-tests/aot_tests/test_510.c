// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test510 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(510) invocation.

#include <stdio.h>

#include "test510_sm.h"

int main(void) {
    test510_t sm;
    test510_init(&sm);
    test510_run(&sm);

    int rc = test510_in_state(&sm, TEST510_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test510: FAIL — active = 0x%08x\n", (unsigned)test510_active_states(&sm));
    }
    test510_destroy(&sm);
    return rc;
}
