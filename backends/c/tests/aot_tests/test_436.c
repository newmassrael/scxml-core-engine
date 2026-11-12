// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//
// W3C SCXML 3.4 + 5.9.2: pure In() inside a parallel region cross-checks a
// sibling region's membership — `In('s1')` reads the bitmap of a state in a
// peer region, validating that <parallel> entry sets every region's leaf bit
// simultaneously (a617a6b6 single-source bitmap migration) so the predicate
// evaluates true at the moment the cond runs

#include <stdio.h>

#include "../require_started.h"
#include "test436_sm.h"

int main(void) {
    test436_t sm;
    REQUIRE_STARTED(test436_init(&sm), TEST436_START_OK);
    test436_run(&sm);

    int rc = test436_ended_in(&sm, TEST436_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test436: FAIL — active = 0x%08x\n", (unsigned)test436_active_states(&sm));
    }
    test436_destroy(&sm);
    return rc;
}
