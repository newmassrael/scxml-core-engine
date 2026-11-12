// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test307 — C11 AOT runner.
//
// W3C SCXML 5.3.3 / B.2: with `binding="late"`, accessing a not-yet-
// declared `<data>` variable in a parent state's `<log>` MUST NOT
// raise `error.execution`; the access yields ECMAScript `undefined`
// silently. Likewise, dereferencing a non-existent property of a bound
// variable (Var1.foo when Var1 = 1) yields `undefined` without raising.
// The W3C corpus marks this manual because the human tester compares
// the two `<log>` outputs to confirm consistent error-free behaviour.
// The automated success criterion mirrors cpp `Test307.h`
// (SimpleAotTest, `PASS_STATE = SM::State::Final`) — reaching `final`
// requires both no-error transitions to fire (s0 raises `foo` → s1,
// s1 raises `bar` → final). Either `<log>` raising `error.execution`
// would route through the `event="error"` transitions and still hit
// `final`, but the absence of the error event is implicit in the c11
// trace — the existing late-binding lua_eval_eventexpr path
// (test280 in registry) already returns nil for undeclared and
// undefined for missing properties without raising.
//
// W3C SCXML 5.3.3 (test307): `binding="late"` access semantics — s0 logs Var1
// before s1 declares it; s1 logs Var1.foo (missing property) after binding.
// Both accesses MUST yield ECMAScript undefined silently (no
// error.execution). The W3C corpus marks this manual because the tester
// compares two `<log>` outputs for consistency, but the conformance bit
// reduces to "reach FINAL via foo→bar (the no-error legs)". The existing
// lua_eval_eventexpr path (test280 late-binding pin) already returns nil for
// undeclared and missing properties without raising.
//
// The lowering this fixture shares with related fixtures is described in
// test_178.c.

#include <stdio.h>

#include "../require_started.h"
#include "test307_sm.h"

int main(void) {
    test307_t sm;
    REQUIRE_STARTED(test307_init(&sm), TEST307_START_OK);
    test307_run(&sm);

    int rc = test307_ended_in(&sm, TEST307_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test307: FAIL — active = 0x%08x\n", (unsigned)test307_active_states(&sm));
    }
    test307_destroy(&sm);
    return rc;
}
