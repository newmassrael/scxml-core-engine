// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test562 — C11 AOT runner.
//
// W3C 5.10 + B.2: `<send><content>this is  a  \nstring</content></send>`
// with the ECMAScript datamodel space-normalizes the multi-whitespace body
// into the single-space string `"this is a string"` before binding to
// `_event.data`. The body's reading — the normalised string — is decided
// at build time (`to_lua_data_content`) and carried as JSON on the event's
// `data[]`, and the receiving
// `cond=_event.data == 'this is a string'` matches pass. Plain `_run` is
// sufficient (no scheduler involved).
//
// W3C SCXML 5.10 + B.2: <send><content>this is  a  \nstring</content></send>
// with the ECMAScript datamodel space-normalizes the multi-whitespace body to
// "this is a string" — the new lua_send_content_literal macro tries Lua eval
// first (fails on plain text), falls back to the `:gsub('%s+', ' ')` +
// match-trim normalization (cpp EventDataHelper::jsonStringToScriptValue
// raw-string fallback)

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "test562_sm.h"

int main(void) {
    test562_t sm;
    REQUIRE_STARTED(test562_init(&sm), TEST562_START_OK);
    test562_run(&sm);

    int rc = test562_ended_in(&sm, TEST562_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test562: FAIL — active = 0x%08x\n", (unsigned)test562_active_states(&sm));
    }
    test562_destroy(&sm);
    return rc;
}
