// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test509 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(509) invocation.

#include <stdio.h>

#include "test509_sm.h"

int main(void) {
    test509_t sm;
    test509_init(&sm);
    test509_run(&sm);

    int rc = test509_in_state(&sm, TEST509_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test509: FAIL — active = 0x%08x\n",
                (unsigned)test509_active_states(&sm));
    }
    test509_destroy(&sm);
    return rc;
}
