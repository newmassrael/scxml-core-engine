// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test577 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(577) invocation.

#include <stdio.h>

#include "test577_sm.h"

int main(void) {
    test577_t sm;
    test577_init(&sm);
    test577_run(&sm);

    int rc = test577_in_state(&sm, TEST577_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test577: FAIL — active = 0x%08x\n",
                (unsigned)test577_active_states(&sm));
    }
    test577_destroy(&sm);
    return rc;
}
