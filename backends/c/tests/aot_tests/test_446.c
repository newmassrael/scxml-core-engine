// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test446 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(446) invocation.

#include <stdio.h>

#include "test446_sm.h"

int main(void) {
    test446_t sm;
    test446_init(&sm);
    test446_run(&sm);

    int rc = test446_in_state(&sm, TEST446_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test446: FAIL — active = 0x%08x\n", (unsigned)test446_active_states(&sm));
    }
    test446_destroy(&sm);
    return rc;
}
