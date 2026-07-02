// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test201 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(201) invocation.

#include <stdio.h>

#include "test201_sm.h"

int main(void) {
    test201_t sm;
    test201_init(&sm);
    test201_run(&sm);

    int rc = test201_in_state(&sm, TEST201_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test201: FAIL — active = 0x%08x\n", (unsigned)test201_active_states(&sm));
    }
    test201_destroy(&sm);
    return rc;
}
