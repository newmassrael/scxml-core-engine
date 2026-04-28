// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test344 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(344) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test344_sm.h"

int main(void) {
    test344_t sm;
    test344_init(&sm);
    test344_run(&sm);

    int rc = test344_in_state(&sm, TEST344_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test344: FAIL — active = 0x%08x\n", (unsigned)test344_active_states(&sm));
    }
    test344_destroy(&sm);
    return rc;
}
