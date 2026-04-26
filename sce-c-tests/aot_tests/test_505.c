// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test505 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(505) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test505_sm.h"

int main(void) {
    test505_t sm;
    test505_init(&sm);
    test505_run(&sm);

    test505_state_t final = test505_get_current_state(&sm);
    int rc = (final == TEST505_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test505: FAIL — final state = %d\n", (int)final);
    }
    test505_destroy(&sm);
    return rc;
}
