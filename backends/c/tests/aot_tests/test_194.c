// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test194 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: an `!`-prefixed `<send target>` literal is
// invalid; the engine raises `error.execution` and skips the
// surrounding entry-action chain, so the second `<send event="timeout">`
// never queues. The internal-priority drain (App.D.2) then matches the
// `error.execution` transition before any wildcard fallback fires.
// Same code path test159 already pins (옵션 A2.target 좁음).

#include <stdio.h>

#include "test194_sm.h"

int main(void) {
    test194_t sm;
    test194_init(&sm);
    test194_run(&sm);

    int rc = test194_in_state(&sm, TEST194_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test194: FAIL — active = 0x%08x\n", (unsigned)test194_active_states(&sm));
    }
    test194_destroy(&sm);
    return rc;
}
