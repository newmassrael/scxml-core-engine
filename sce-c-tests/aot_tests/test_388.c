// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test388 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(388) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test388_sm.h"

int main(void) {
    test388_t sm;
    test388_init(&sm);
    test388_run(&sm);

    int rc = test388_in_state(&sm, TEST388_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test388: FAIL — active = 0x%08x\n", (unsigned)test388_active_states(&sm));
    }
    test388_destroy(&sm);
    return rc;
}
