// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test310 — C11 AOT runner.
//
// W3C SCXML 5.9.2: pure In() predicate cond — `<transition cond="In('s1')"
// target="pass"/>` from s0 fires only after the eventless walk has
// already entered s1 (via the `<state id="s0"><transition target="s1"/>`
// hop), so the cond evaluation must read the live configuration bitmap
// at the moment of the second eventless macrostep. Native C11 lowering
// (filter `to_in_predicate_c11`) routes In('s1') → `test310_in_state(sm,
// TEST310_STATE_S1)` so the bitmap read happens directly through the
// public predicate — no lua trampoline, no runtime symbol resolution
// (mirrors cpp `parser::convert_in_to_cpp` of `this->isStateActive`).
//
// W3C SCXML 5.9.2: pure In('s1') predicate cond is native-lowered (no
// datamodel, no script engine) — `to_in_predicate_c11` filter rewrites
// In('s1') → test310_in_state(sm, TEST310_STATE_S1) at codegen time,
// mirroring cpp parser::convert_in_to_cpp's this->isStateActive("s1")
// substitution; the eventless walk reaches s1 first so the bitmap-read
// predicate returns true and the s0 transition routes to pass

#include <stdio.h>

#include "../require_started.h"
#include "test310_sm.h"

int main(void) {
    test310_t sm;
    REQUIRE_STARTED(test310_init(&sm), TEST310_START_OK);
    test310_run(&sm);

    int rc = test310_ended_in(&sm, TEST310_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test310: FAIL — active = 0x%08x\n", (unsigned)test310_active_states(&sm));
    }
    test310_destroy(&sm);
    return rc;
}
