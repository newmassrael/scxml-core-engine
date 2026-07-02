// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test520 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(520) invocation.

#include <stdio.h>

#include "test520_sm.h"

int main(void) {
    test520_t sm;
    test520_init(&sm);
    test520_run(&sm);

    int rc = test520_in_state(&sm, TEST520_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test520: FAIL — active = 0x%08x\n", (unsigned)test520_active_states(&sm));
    }
    test520_destroy(&sm);
    return rc;
}
