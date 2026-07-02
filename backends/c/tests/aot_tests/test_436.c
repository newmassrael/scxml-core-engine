// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test436 — C11 AOT runner.
//
// W3C SCXML 3.4 + 5.9.2: pure In() predicate inside a parallel region.
// The fixture has `<parallel id="ps">` with two regions; the second
// region's transition `cond="In('s1')"` reads membership of a sibling
// region's state from the active configuration bitmap. Native C11 lower
// (filter `to_in_predicate_c11`) issues `test436_in_state(sm, TEST436_
// STATE_S1)` directly against the bitmap, mirroring cpp's `this->
// isStateActive("s1")` semantics — no lua side trampoline needed
// (T3 inline-only lock-in for C11).

#include <stdio.h>

#include "test436_sm.h"

int main(void) {
    test436_t sm;
    test436_init(&sm);
    test436_run(&sm);

    int rc = test436_in_state(&sm, TEST436_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test436: FAIL — active = 0x%08x\n", (unsigned)test436_active_states(&sm));
    }
    test436_destroy(&sm);
    return rc;
}
