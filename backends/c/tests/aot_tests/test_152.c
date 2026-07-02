// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test152 — C11 AOT runner.
//
// W3C SCXML 4.6: An ill-formed <foreach> (missing the required `array`
// or `item` attribute) must raise error.execution AND must NOT execute
// any of its body. test152 has two variants back-to-back:
//   s0: <foreach item="Var2" index="Var3">  — no `array` attribute
//   s1: <foreach index="Var3" array="Var5"> — no `item` attribute
// Both bodies contain `<assign location="Var1" expr="Var1 + 1"/>`. The
// pass guard at s2 checks `cond="Var1 == 0"`, proving the body never
// ran in either variant. The transitions on `event="error.execution"`
// (s0→s1, s1→s2) confirm the raise actually reaches the queue.
//
// First consumer of the codegen-time foreach validation path: when the
// parser observes empty `array`/`item` attributes (preserved as empty
// strings via `unwrap_or("")`), the C11 emit_action elides the foreach
// body entirely and emits a single EVENT_ERROR_EXECUTION raise. The
// EVENT_ERROR_EXECUTION enum value is auto-registered into model.events
// by analyzer.rs::apply_script_engine_implications (every <foreach>
// triggers needs_script_engine).

#include <stdio.h>

#include "test152_sm.h"

int main(void) {
    test152_t sm;
    test152_init(&sm);
    test152_run(&sm);

    int rc = test152_in_state(&sm, TEST152_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test152: FAIL — active = 0x%08x\n", (unsigned)test152_active_states(&sm));
    }
    test152_destroy(&sm);
    return rc;
}
