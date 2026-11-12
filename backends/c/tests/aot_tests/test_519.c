// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test519 — C11 AOT runner.
//
// W3C SCXML C.2 (test519): `<send event="test" target="..."
// type="BasicHTTP"><param name="param1" expr="1"/></send>` — event +
// static-literal numeric param. Body = `_scxmleventname=test&param1=1`. The
// standalone server's test519-specific numeric coercion
// (`tests/w3c/standalone_http_server.js:65-67`, `Number(value)` ≠ NaN)
// returns `data: {"param1": 1, "_scxmleventname": "test"}`. The transition
// `expr="_event.data[\"param1\"] == 1"` is recognised by
// `parser.rs:1059-1063` as a `cond=` synonym (all backends share the
// `attribute("cond").or_else("expr")` fall-through), and the codegen lowers
// it to a `luaL_dostring` returning a boolean — so
// `model.needs_script_engine` is on and `sce_test_lua_push_json` promotes the
// JSON object onto `_event.data` before the cond evaluates. Distinct from
// test567 by reading `_event.data` directly in the transition cond without an
// intermediate `<assign>`/datamodel hop, exercising the cond+JSON-deref path
// against numeric truthiness.

#include <stdio.h>

#include "../require_started.h"
#include "basic_http_test_endpoint.h"
#include "test519_sm.h"

int main(void) {
    test519_t sm;
    char access_uri[SCE_W3C_HTTP_URI_MAX];
    /* W3C SCXML C.2.3: the ctest fixture owns the inbound listener, and
       basic_http_test_endpoint.h is the one place that says where it answers.
       The fixture script, the gates and this runner all read it, so the bind
       address and the published 'location' cannot come apart. The converted
       W3C document reads `_ioprocessors['basichttp'].location` to address its
       send, so a machine initialised through plain `_init` would publish no
       entry and send nowhere. */
    REQUIRE_STARTED(test519_init_with_basic_http(&sm, sce_w3c_http_test_access_uri(access_uri, sizeof access_uri)),
                    TEST519_START_OK);
    test519_run(&sm);

    int rc = test519_ended_in(&sm, TEST519_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test519: FAIL — active = 0x%08x\n", (unsigned)test519_active_states(&sm));
    }
    test519_destroy(&sm);
    return rc;
}
