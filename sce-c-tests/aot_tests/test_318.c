// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test318 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(318) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test318_sm.h"

int main(void) {
    test318_t sm;
    test318_init(&sm);
    test318_run(&sm);

    test318_state_t final = test318_get_current_state(&sm);
    int rc = (final == TEST318_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test318: FAIL — final state = %d\n", (int)final);
    }
    test318_destroy(&sm);
    return rc;
}
