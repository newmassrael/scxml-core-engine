// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test513 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(513) invocation.

#include <stdio.h>

#include "test513_sm.h"

int main(void) {
    test513_t sm;
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener
       (tests/w3c/http_server_fixture.sh binds localhost:8080/test), so this
       runner declares that address as the machine's published BasicHTTP
       'location'. The fixture reads
       `_ioprocessors['basichttp'].location` to address its send, so a machine
       initialised through plain `_init` would publish no entry and send
       nowhere. */
    test513_init_with_basic_http(&sm, "http://localhost:8080/test");
    test513_run(&sm);

    int rc = test513_in_state(&sm, TEST513_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test513: FAIL — active = 0x%08x\n", (unsigned)test513_active_states(&sm));
    }
    test513_destroy(&sm);
    return rc;
}
