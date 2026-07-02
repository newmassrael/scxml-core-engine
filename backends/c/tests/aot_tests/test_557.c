// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test557 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(557) invocation.

#include <stdio.h>

#include "test557_sm.h"

int main(void) {
    test557_t sm;
    test557_init(&sm);
    test557_run(&sm);

    int rc = test557_in_state(&sm, TEST557_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test557: FAIL — active = 0x%08x\n", (unsigned)test557_active_states(&sm));
    }
    test557_destroy(&sm);
    return rc;
}
