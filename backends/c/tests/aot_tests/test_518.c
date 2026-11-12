// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test518 — C11 AOT runner.
//
// W3C SCXML C.2 (test518): `<send event="test" target="..." namelist="Var1"
// type="BasicHTTP"/>` — namelist evaluation. The arm walks `action.namelist |
// split` and emits a `lua_eval_to_string` per variable so each name's current
// Lua value is appended as a form parameter (`Var1=2`). `<datamodel><data
// id="Var1" expr="2"/></datamodel>` triggers `needs_script_engine = true`, so
// the Lua state is initialised and the eval succeeds. The transition
// `event="test" target="pass"` does not deref `_event.data`, but the data
// promotion still fires because `model.needs_script_engine` is on (harmless
// dead-write — cpp's lane behaves identically).

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test518_sm.h"

int main(void) {
    test518_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test518_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST518_START_OK);
    test518_run(&sm);

    int rc = test518_ended_in(&sm, TEST518_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test518: FAIL — active = 0x%08x\n", (unsigned)test518_active_states(&sm));
    }
    test518_destroy(&sm);
    return rc;
}
