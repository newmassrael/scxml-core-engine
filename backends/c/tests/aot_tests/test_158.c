// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test158 — C11 AOT runner.
//
// W3C SCXML 4.5: Executable content inside an <onentry> block runs in
// document order. test158's s0 onentry raises event1 then event2; the
// internal queue must observe that order so s0's `event="event1"`
// transition fires first (routing to s1), and s1's `event="event2"`
// transition then routes to pass. Any reordering by the engine would
// land the wildcard `event="*" target="fail"` arm instead.
//
// Fixture-only landing: covered entirely by infrastructure already
// emitted by 31750729 (raise emit) — the existing emit_action raise
// branch pushes events in document order and the FIFO event queue
// preserves it. NEEDS_LUA holds because the document declares
// `<data expr="0"/>`, which routes through the Lua datamodel.

#include <stdio.h>

#include "test158_sm.h"

int main(void) {
    test158_t sm;
    test158_init(&sm);
    test158_run(&sm);

    int rc = test158_in_state(&sm, TEST158_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test158: FAIL — active = 0x%08x\n", (unsigned)test158_active_states(&sm));
    }
    test158_destroy(&sm);
    return rc;
}
