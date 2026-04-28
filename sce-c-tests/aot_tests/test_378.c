// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test378 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(378) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test378_sm.h"

int main(void) {
    test378_t sm;
    test378_init(&sm);
    test378_run(&sm);

    int rc = test378_in_state(&sm, TEST378_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test378: FAIL — active = 0x%08x\n", (unsigned)test378_active_states(&sm));
    }
    test378_destroy(&sm);
    return rc;
}
