// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test532 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(532) invocation.

#include <stdio.h>

#include "test532_sm.h"

int main(void) {
    test532_t sm;
    test532_init(&sm);
    test532_run(&sm);

    int rc = test532_in_state(&sm, TEST532_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test532: FAIL — active = 0x%08x\n", (unsigned)test532_active_states(&sm));
    }
    test532_destroy(&sm);
    return rc;
}
