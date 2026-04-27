// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test322 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(322) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test322_sm.h"

int main(void) {
    test322_t sm;
    test322_init(&sm);
    test322_run(&sm);

    int rc = test322_in_state(&sm, TEST322_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test322: FAIL — active = 0x%08x\n", (unsigned)test322_active_states(&sm));
    }
    test322_destroy(&sm);
    return rc;
}
