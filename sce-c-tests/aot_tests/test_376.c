// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test376 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(376) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test376_sm.h"

int main(void) {
    test376_t sm;
    test376_init(&sm);
    test376_run(&sm);

    int rc = test376_in_state(&sm, TEST376_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test376: FAIL — active = 0x%08x\n", (unsigned)test376_active_states(&sm));
    }
    test376_destroy(&sm);
    return rc;
}
