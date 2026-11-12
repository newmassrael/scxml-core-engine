// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test534 — C11 AOT runner.
//
// W3C SCXML C.2 (test534): `<send event="test" target="..."
// type="BasicHTTP"><content>some content</content></send>` — event AND
// content together. cpp's `HttpEventTarget::send`
// (sce/src/events/HttpEventTarget.cpp:47) prefers form-urlencoded when an
// event name is present; the C11 arm mirrors this — the `{% if action.event
// or action.params or action.namelist %}` branch wins, so the body becomes
// `_scxmleventname=test` and the content is silently ignored (matching the
// comment in the fixture XML: "this content will be ignored, but it's here to
// make sure we have a message body").

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test534_sm.h"

int main(void) {
    test534_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test534_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST534_START_OK);
    test534_run(&sm);

    int rc = test534_ended_in(&sm, TEST534_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test534: FAIL — active = 0x%08x\n", (unsigned)test534_active_states(&sm));
    }
    test534_destroy(&sm);
    return rc;
}
