// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test403a — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(403a) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test403a_sm.h"

int main(void) {
    test403a_t sm;
    test403a_init(&sm);
    test403a_run(&sm);

    test403a_state_t final = test403a_get_current_state(&sm);
    int rc = (final == TEST403A_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test403a: FAIL — final state = %d\n", (int)final);
    }
    test403a_destroy(&sm);
    return rc;
}
