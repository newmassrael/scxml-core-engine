// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test309 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(309) invocation. The
// runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST309_STATE_PASS, return 0 on PASS / 1 on
// FAIL. Per-fixture conditional compilation (lua54 link, etc.) is
// already handled by the harness CMake rule.

#include <stdio.h>

#include "test309_sm.h"

int main(void) {
    test309_t sm;
    test309_init(&sm);
    test309_run(&sm);

    int rc = test309_in_state(&sm, TEST309_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test309: FAIL — active = 0x%08x\n", (unsigned)test309_active_states(&sm));
    }
    test309_destroy(&sm);
    return rc;
}
