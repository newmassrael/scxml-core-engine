// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test387 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(387) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test387_sm.h"

int main(void) {
    test387_t sm;
    test387_init(&sm);
    test387_run(&sm);

    int rc = test387_in_state(&sm, TEST387_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test387: FAIL — active = 0x%08x\n", (unsigned)test387_active_states(&sm));
    }
    test387_destroy(&sm);
    return rc;
}
