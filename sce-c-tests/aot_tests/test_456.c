// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test456 — C11 AOT runner.
//
// W3C SCXML 5.8: inline `<script>` body can update an existing
// datamodel variable. test456 declares `<data id="Var1" expr="0"/>`
// at the document root, then s0's onentry runs `<script>Var1+=1</script>`
// (compound assignment). After the block Var1 is 1, and the guarded
// transition `cond="Var1 == 1"` matches.
//
// The compound `+=` lowers to `Var1 = Var1 + (1)` via
// `lua_transformer::transform_compound_assignment` — the same pass
// that handles `<assign expr="Var1+=1"/>` for non-script sites. The
// inline-script emit path lives in `state_machine.c.jinja2::emit_action`'s
// `script` branch (sibling of the `assign` branch).

#include <stdio.h>

#include "test456_sm.h"

int main(void) {
    test456_t sm;
    test456_init(&sm);
    test456_run(&sm);

    int rc = test456_in_state(&sm, TEST456_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        (void)fprintf(stderr, "test456: FAIL — active = 0x%08x\n", (unsigned)test456_active_states(&sm));
    }
    test456_destroy(&sm);
    return rc;
}
