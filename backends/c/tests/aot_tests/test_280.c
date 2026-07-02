// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test280 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(280) invocation. The
// runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST280_STATE_PASS, return 0 on PASS / 1 on
// FAIL. Per-fixture conditional compilation (lua54 link, etc.) is
// already handled by the harness CMake rule.

#include <stdio.h>

#include "test280_sm.h"

int main(void) {
    test280_t sm;
    test280_init(&sm);
    test280_run(&sm);

    int rc = test280_in_state(&sm, TEST280_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test280: FAIL — active = 0x%08x\n", (unsigned)test280_active_states(&sm));
    }
    test280_destroy(&sm);
    return rc;
}
