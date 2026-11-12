// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test578 — C11 AOT runner.
//
// W3C 5.10 + B.2: `<send><content>{ "productName": "bar", "size": 27 }</content></send>`
// with the ECMAScript datamodel parses the JSON object body and binds the
// result as a structured value on `_event.data`. The body's reading is
// decided at build time (`to_lua_data_content`), evaluated at send time,
// and carried as JSON on the event's `data[]` to the receiving
// transition; the cond
// `_event.data.productName == 'bar'` reads the table field via the
// ECMAScript-via-Lua datamodel and matches pass.
//
// W3C SCXML 5.10 + B.2:
// <send><content>{"productName":"bar","size":27}</content></send> parses the
// JSON body via the lua_send_content_literal JSON-key-syntax shim (`"key":` →
// `["key"]=`) + pcall(load(...)); the resulting Lua table lands on
// _event.data so the receiver's cond `_event.data.productName == 'bar'` reads
// the structured field and routes to pass

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "test578_sm.h"

int main(void) {
    test578_t sm;
    REQUIRE_STARTED(test578_init(&sm), TEST578_START_OK);
    test578_run(&sm);

    int rc = test578_ended_in(&sm, TEST578_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test578: FAIL — active = 0x%08x\n", (unsigned)test578_active_states(&sm));
    }
    test578_destroy(&sm);
    return rc;
}
