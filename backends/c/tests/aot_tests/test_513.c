// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test513 — C11 AOT runner.
//
// W3C SCXML C.2 (test513): event-only POST; verifies the server replies with
// HTTP 200 OK (test runner does not inspect the status code, but the
// reception of the echoed `event` is the implicit verification — the C11
// client treats only 2xx-with-body as success-equivalent because
// `sce_test_http_parse_response` requires a JSON object). Same emit shape as
// test201.

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test513_sm.h"

int main(void) {
    test513_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The fixture
       reads `_ioprocessors['basichttp'].location` to address its send, so a
       machine initialised through plain `_init` would publish no entry and
       send nowhere. */
    REQUIRE_STARTED(test513_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST513_START_OK);
    test513_run(&sm);

    int rc = test513_ended_in(&sm, TEST513_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test513: FAIL — active = 0x%08x\n", (unsigned)test513_active_states(&sm));
    }
    test513_destroy(&sm);
    return rc;
}
