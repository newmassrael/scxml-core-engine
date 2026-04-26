// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test550 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(550) invocation.

#include <stdio.h>

#include "test550_sm.h"

int main(void) {
    test550_t sm;
    test550_init(&sm);
    test550_run(&sm);

    test550_state_t final = test550_get_current_state(&sm);
    int rc = (final == TEST550_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test550: FAIL — final state = %d\n", (int)final);
    }
    test550_destroy(&sm);
    return rc;
}
