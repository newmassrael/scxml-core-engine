// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test567 — C11 AOT runner.
//
// W3C SCXML C.2 (test567): `<send event="test" target="..."
// type="BasicHTTP"><param name="param1" expr="2"/></send>` — event +
// static-literal param. Body = `_scxmleventname=test&param1=2`. The
// standalone server returns `data: {"param1": 2, "_scxmleventname": "test"}`
// — a JSON object. The codegen's `model.needs_script_engine` gate is on
// (because the transition action `<assign location="Var1"
// expr="_event.data.param1"/>` triggers `AssignAction` cause), so the object
// span rides on the reply event's `data[]` and its dequeue's
// `set_current_event` decodes it onto `_event.data`, the assign reads
// `_event.data.param1` as the Lua integer 2, the s1 cond `Var1 == 2`
// evaluates true → s1→pass.

#include <stdio.h>

#include "../require_started.h"
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
    REQUIRE_STARTED(test567_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST567_START_OK);
    test567_run(&sm);

    int rc = test567_ended_in(&sm, TEST567_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test567: FAIL — active = 0x%08x\n", (unsigned)test567_active_states(&sm));
    }
    test567_destroy(&sm);
    return rc;
}
