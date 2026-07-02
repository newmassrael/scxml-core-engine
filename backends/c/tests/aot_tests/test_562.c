// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test562 — C11 AOT runner.
//
// W3C 5.10 + B.2: `<send><content>this is  a  \nstring</content></send>`
// with the ECMAScript datamodel space-normalizes the multi-whitespace body
// into the single-space string `"this is a string"` before binding to
// `_event.data`. The new `lua_send_content_literal` macro tries the body
// as a Lua expression first (load+pcall) — for plain text this fails, the
// fallback gsub `%s+ → ' '` collapse + match-trim runs, and the receiving
// `cond=_event.data == 'this is a string'` matches pass. Plain `_run` is
// sufficient (no scheduler involved).
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(562) invocation.

#include <stdint.h>
#include <stdio.h>

#include "test562_sm.h"

int main(void) {
    test562_t sm;
    test562_init(&sm);
    test562_run(&sm);

    int rc = test562_in_state(&sm, TEST562_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test562: FAIL — active = 0x%08x\n", (unsigned)test562_active_states(&sm));
    }
    test562_destroy(&sm);
    return rc;
}
