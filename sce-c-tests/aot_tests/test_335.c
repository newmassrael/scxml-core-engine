// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test335 — C11 AOT runner.
//
// W3C SCXML 5.10.1: `_event.origin` is bound only on external pops (process_event_queues seeds `_pending_event_origin =
// '<name>_session'`), so internal events leave `_event.origin` as nil — the receiving transition's cond `typeof
// _event.origin === 'undefined'` (lua_transformer rewrites to `_event.origin == nil`) matches and routes to pass.
// Mirrors 옵션 σ + τ + υ binding.

#include <stdio.h>

#include "test335_sm.h"

int main(void) {
    test335_t sm;
    test335_init(&sm);
    test335_run(&sm);

    int rc = test335_in_state(&sm, TEST335_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test335: FAIL — active = 0x%08x\n", (unsigned)test335_active_states(&sm));
    }
    test335_destroy(&sm);
    return rc;
}
