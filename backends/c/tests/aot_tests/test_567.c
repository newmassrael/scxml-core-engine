// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test567 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(567) invocation.

#include <stdio.h>

#include "basic_http_test_endpoint.h"
#include "test567_sm.h"

int main(void) {
    test567_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    test567_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri));
    test567_run(&sm);

    int rc = test567_in_state(&sm, TEST567_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test567: FAIL — active = 0x%08x\n", (unsigned)test567_active_states(&sm));
    }
    test567_destroy(&sm);
    return rc;
}
