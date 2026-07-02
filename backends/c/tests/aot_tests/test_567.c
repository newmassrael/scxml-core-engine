// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test567 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(567) invocation.

#include <stdio.h>

#include "test567_sm.h"

int main(void) {
    test567_t sm;
    test567_init(&sm);
    test567_run(&sm);

    int rc = test567_in_state(&sm, TEST567_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test567: FAIL — active = 0x%08x\n", (unsigned)test567_active_states(&sm));
    }
    test567_destroy(&sm);
    return rc;
}
