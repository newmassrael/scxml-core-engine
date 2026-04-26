// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test407 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(407) invocation.

#include <stdio.h>

#include "test407_sm.h"

int main(void) {
    test407_t sm;
    test407_init(&sm);
    test407_run(&sm);

    test407_state_t final = test407_get_current_state(&sm);
    int rc = (final == TEST407_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test407: FAIL — final state = %d\n", (int)final);
    }
    test407_destroy(&sm);
    return rc;
}
