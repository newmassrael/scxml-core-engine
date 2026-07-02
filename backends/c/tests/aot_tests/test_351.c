// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test351 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(351) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test351_sm.h"

int main(void) {
    test351_t sm;
    test351_init(&sm);
    test351_run(&sm);

    int rc = test351_in_state(&sm, TEST351_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test351: FAIL — active = 0x%08x\n", (unsigned)test351_active_states(&sm));
    }
    test351_destroy(&sm);
    return rc;
}
