// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test457 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(457) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test457_sm.h"

int main(void) {
    test457_t sm;
    test457_init(&sm);
    test457_run(&sm);

    int rc = test457_in_state(&sm, TEST457_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test457: FAIL — active = 0x%08x\n", (unsigned)test457_active_states(&sm));
    }
    test457_destroy(&sm);
    return rc;
}
