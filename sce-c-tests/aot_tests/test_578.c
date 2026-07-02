// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test578 — C11 AOT runner.
//
// W3C 5.10 + B.2: `<send><content>{ "productName": "bar", "size": 27 }</content></send>`
// with the ECMAScript datamodel parses the JSON object body and binds the
// result as a structured value on `_event.data`. The new
// `lua_send_content_literal` macro applies a JSON-key-syntax shim
// (`"key":` → `["key"]=`) so the body becomes a valid Lua table literal,
// then `pcall(load(...))` evaluates it and `_pending_donedata` carries
// the table across to the receiving transition; the cond
// `_event.data.productName == 'bar'` reads the table field via the
// ECMAScript-via-Lua datamodel and matches pass.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(578) invocation.

#include <stdint.h>
#include <stdio.h>

#include "test578_sm.h"

int main(void) {
    test578_t sm;
    test578_init(&sm);
    test578_run(&sm);

    int rc = test578_in_state(&sm, TEST578_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test578: FAIL — active = 0x%08x\n", (unsigned)test578_active_states(&sm));
    }
    test578_destroy(&sm);
    return rc;
}
