// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test451 — C11 AOT runner.
//
// W3C SCXML 5.9.2 / B.2: ECMAScript-style In() predicate is supported
// alongside the native `In('s1')` form (parser strips the surrounding
// parentheses chain via `is_pure_in_predicate`). The C11 native lower
// (filter `to_in_predicate_c11`) accepts both shapes because the regex
// matches `In\(['"]([^'"]+)['"]\)` regardless of any wrapping group, so
// the produced `test451_in_state(sm, TEST451_STATE_S1)` chunk matches
// cpp's `this->isStateActive("s1")` semantics with no runtime resolver.
//
// W3C SCXML 5.9.2 + B.2: ECMAScript-syntax pure In() predicate with
// surrounding parens (e.g. `(In('s1'))`) routes through the same native-lower
// filter — the `is_pure_in_predicate` parser predicate already strips
// wrapping parens, and the regex `In\(['"]([^'"]+)['"]\)` matches regardless
// of the outer grouping, so the produced chunk is identical to test310's

#include <stdio.h>

#include "../require_started.h"
#include "test451_sm.h"

int main(void) {
    test451_t sm;
    REQUIRE_STARTED(test451_init(&sm), TEST451_START_OK);
    test451_run(&sm);

    int rc = test451_ended_in(&sm, TEST451_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test451: FAIL — active = 0x%08x\n", (unsigned)test451_active_states(&sm));
    }
    test451_destroy(&sm);
    return rc;
}
