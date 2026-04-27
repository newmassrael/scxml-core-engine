// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test445 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(445) invocation.

#include <stdio.h>

#include "test445_sm.h"

int main(void) {
    test445_t sm;
    test445_init(&sm);
    test445_run(&sm);

    int rc = test445_in_state(&sm, TEST445_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test445: FAIL — active = 0x%08x\n", (unsigned)test445_active_states(&sm));
    }
    test445_destroy(&sm);
    return rc;
}
