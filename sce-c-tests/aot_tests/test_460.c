// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test460 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(460) invocation. The
// runner here is the standard C11 boilerplate.

#include <stdio.h>

#include "test460_sm.h"

int main(void) {
    test460_t sm;
    test460_init(&sm);
    test460_run(&sm);

    int rc = test460_in_state(&sm, TEST460_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test460: FAIL — active = 0x%08x\n", (unsigned)test460_active_states(&sm));
    }
    test460_destroy(&sm);
    return rc;
}
