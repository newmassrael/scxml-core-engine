// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test321 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(321) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test321_sm.h"

int main(void) {
    test321_t sm;
    test321_init(&sm);
    test321_run(&sm);

    test321_state_t final = test321_get_current_state(&sm);
    int rc = (final == TEST321_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test321: FAIL — final state = %d\n", (int)final);
    }
    test321_destroy(&sm);
    return rc;
}
