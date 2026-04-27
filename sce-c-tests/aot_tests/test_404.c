// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test404 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(404) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test404_sm.h"

int main(void) {
    test404_t sm;
    test404_init(&sm);
    test404_run(&sm);

    int rc = test404_in_state(&sm, TEST404_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test404: FAIL — active = 0x%08x\n", (unsigned)test404_active_states(&sm));
    }
    test404_destroy(&sm);
    return rc;
}
