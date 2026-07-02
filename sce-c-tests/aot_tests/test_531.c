// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test531 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(531) invocation.

#include <stdio.h>

#include "test531_sm.h"

int main(void) {
    test531_t sm;
    test531_init(&sm);
    test531_run(&sm);

    int rc = test531_in_state(&sm, TEST531_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test531: FAIL — active = 0x%08x\n", (unsigned)test531_active_states(&sm));
    }
    test531_destroy(&sm);
    return rc;
}
