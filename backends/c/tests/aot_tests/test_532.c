// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test532 — C11 AOT runner.
//
// W3C SCXML C.2 (test532): event-only POST shape identical to test201; the
// fixture's transition uses an `expr="..."` attribute (a non-standard typo
// for `cond=` in the W3C corpus) which the parser treats as inert — no
// datamodel deref happens. The `event="test"` match falls through to
// `target="pass"` regardless of the expr value, so the codegen does not need
// to expose `_event.data` for this fixture. Same emit shape as test201.

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test532_sm.h"

int main(void) {
    test532_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test532_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST532_START_OK);
    test532_run(&sm);

    int rc = test532_ended_in(&sm, TEST532_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test532: FAIL — active = 0x%08x\n", (unsigned)test532_active_states(&sm));
    }
    test532_destroy(&sm);
    return rc;
}
