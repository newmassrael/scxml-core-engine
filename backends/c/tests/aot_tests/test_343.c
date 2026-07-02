// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test343 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(343) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test343_sm.h"

int main(void) {
    test343_t sm;
    test343_init(&sm);
    test343_run(&sm);

    int rc = test343_in_state(&sm, TEST343_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test343: FAIL — active = 0x%08x\n", (unsigned)test343_active_states(&sm));
    }
    test343_destroy(&sm);
    return rc;
}
