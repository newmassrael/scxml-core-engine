// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test459 — C11 AOT runner.
//
// W3C SCXML 4.4: <log label="..." expr="..."> is observable side-effect
// only — the spec mandates no statechart effect, just developer-facing
// output. test459 is the first fixture forcing emit_action's <log>
// branch: the pass / fail finals each carry an onentry log call, so once
// the fixture compiles cleanly under -Werror the new fprintf emit is
// validated against the C11 dialect (mirrors cpp `actions/log.jinja2`
// via `lua_eval_eventexpr` reuse — the macro evaluates any expression
// to its `tostring()` form, not just <send eventexpr>).
//
// The boolean PASS path is independent of the log emit: s0's foreach
// over Var4=[1,2,3] iterates in doc order, accumulating each strictly
// larger value into Var1 (1 → 2 → 3) without ever flipping Var5 to 0,
// and the index Var3 lands at 2. The transition cond
// `Var4==0 | Var3 != 2` lowers through the Lua transformer's
// `parenthesize_bitwise_operands` shim into `(Var4==0) | (Var3 ~= 2)` —
// both operands evaluate to false, the `|` chunk fails (Lua 5.4 does
// not coerce booleans to integers for bitwise ops), `lua_eval_guard`
// silent-drops to `_trans_pass = false`, and the fall-through transition
// to `pass` fires.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(459) invocation.

#include <stdio.h>

#include "test459_sm.h"

int main(void) {
    test459_t sm;
    test459_init(&sm);
    test459_run(&sm);

    int rc = test459_in_state(&sm, TEST459_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test459: FAIL — active = 0x%08x\n",
                (unsigned)test459_active_states(&sm));
    }
    test459_destroy(&sm);
    return rc;
}
