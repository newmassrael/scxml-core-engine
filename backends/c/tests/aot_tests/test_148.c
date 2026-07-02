// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test148 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(148) invocation. The
// runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST148_STATE_PASS, return 0 on PASS / 1 on
// FAIL. Per-fixture conditional compilation (lua54 link, etc.) is
// already handled by the harness CMake rule.

#include <stdio.h>

#include "test148_sm.h"

int main(void) {
    test148_t sm;
    test148_init(&sm);
    test148_run(&sm);

    int rc = test148_in_state(&sm, TEST148_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test148: FAIL — active = 0x%08x\n", (unsigned)test148_active_states(&sm));
    }
    test148_destroy(&sm);
    return rc;
}
