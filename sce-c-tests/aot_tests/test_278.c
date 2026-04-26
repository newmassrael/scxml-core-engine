// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test278 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(278) invocation.

#include <stdio.h>

#include "test278_sm.h"

int main(void) {
    test278_t sm;
    test278_init(&sm);
    test278_run(&sm);

    test278_state_t final = test278_get_current_state(&sm);
    int rc = (final == TEST278_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test278: FAIL — final state = %d\n", (int)final);
    }
    test278_destroy(&sm);
    return rc;
}
