// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test520 — C11 AOT runner.
//
// W3C SCXML C.2 (test520): `<send target="..." type="BasicHTTP"><content>this
// is some content</content></send>` — content-only POST (text/plain). No
// event/params/namelist → the `{% elif action.content %}` branch in
// `state_machine.c.jinja2` emits the literal as a static C-string into the
// body buffer with `_http_content_type = "text/plain"`. The standalone
// server's content-type-fallback returns event="HTTP.POST" (the HTTP method
// name) per its `event1` / `HTTP.POST` default ladder.

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test520_sm.h"

int main(void) {
    test520_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test520_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST520_START_OK);
    test520_run(&sm);

    int rc = test520_ended_in(&sm, TEST520_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test520: FAIL — active = 0x%08x\n", (unsigned)test520_active_states(&sm));
    }
    test520_destroy(&sm);
    return rc;
}
