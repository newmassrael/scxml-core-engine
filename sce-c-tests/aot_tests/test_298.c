// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test298 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(298) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test298_sm.h"

int main(void) {
    test298_t sm;
    test298_init(&sm);
    test298_run(&sm);

    int rc = test298_in_state(&sm, TEST298_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test298: FAIL — active = 0x%08x\n", (unsigned)test298_active_states(&sm));
    }
    test298_destroy(&sm);
    return rc;
}
