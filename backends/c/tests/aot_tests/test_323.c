// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test323 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(323) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test323_sm.h"

int main(void) {
    test323_t sm;
    test323_init(&sm);
    test323_run(&sm);

    int rc = test323_in_state(&sm, TEST323_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test323: FAIL — active = 0x%08x\n", (unsigned)test323_active_states(&sm));
    }
    test323_destroy(&sm);
    return rc;
}
