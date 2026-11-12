// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test201 — C11 AOT runner.
//
// W3C SCXML C.2 (test201): `<send type="BasicHTTP"
// target="http://localhost:8080/test" event="event1"/>` event-only POST. The
// standalone Node server (`tests/w3c/standalone_http_server.js`) JSON-echoes
// the request as `{"event":"event1","data":...}`; the BasicHTTP+target arm in
// `state_machine.c.jinja2` builds a form-encoded body
// (`_scxmleventname=event1`), calls `sce_test_http_post` synchronously,
// parses the response via `sce_test_http_parse_response`, dispatches the
// echoed event name through the codegen-emitted `_event_from_name` switch,
// and raises onto the external queue. No datamodel/cond → no Lua link
// required, `_event.data` promotion skipped. cpp
// `actions/send.jinja2:347-428` performHttpSend() callback + Go
// `backends/go/tests/harness/harness.go::SetupHTTPTest` are the reference
// implementations.

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test201_sm.h"

int main(void) {
    test201_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test201_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST201_START_OK);
    test201_run(&sm);

    int rc = test201_ended_in(&sm, TEST201_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test201: FAIL — active = 0x%08x\n", (unsigned)test201_active_states(&sm));
    }
    test201_destroy(&sm);
    return rc;
}
