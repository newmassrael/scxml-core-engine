// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test405 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(405) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test405_sm.h"

int main(void) {
    test405_t sm;
    test405_init(&sm);
    test405_run(&sm);

    int rc = test405_in_state(&sm, TEST405_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test405: FAIL — active = 0x%08x\n", (unsigned)test405_active_states(&sm));
    }
    test405_destroy(&sm);
    return rc;
}
