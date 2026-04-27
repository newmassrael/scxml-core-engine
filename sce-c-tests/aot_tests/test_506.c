// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test506 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(506) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test506_sm.h"

int main(void) {
    test506_t sm;
    test506_init(&sm);
    test506_run(&sm);

    int rc = test506_in_state(&sm, TEST506_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test506: FAIL — active = 0x%08x\n", (unsigned)test506_active_states(&sm));
    }
    test506_destroy(&sm);
    return rc;
}
