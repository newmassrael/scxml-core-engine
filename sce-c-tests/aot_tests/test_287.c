// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test287 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(287) invocation. The
// runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST287_STATE_PASS, return 0 on PASS / 1 on
// FAIL. Per-fixture conditional compilation (lua54 link, etc.) is
// already handled by the harness CMake rule.

#include <stdio.h>

#include "test287_sm.h"

int main(void) {
    test287_t sm;
    test287_init(&sm);
    test287_run(&sm);

    test287_state_t final = test287_get_current_state(&sm);
    int rc = (final == TEST287_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test287: FAIL — final state = %d\n", (int)final);
    }
    test287_destroy(&sm);
    return rc;
}
