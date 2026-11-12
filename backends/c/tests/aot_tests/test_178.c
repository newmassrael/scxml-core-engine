// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test178 — C11 AOT runner.
//
// W3C SCXML 6.2: `<send event="event1">` with two `<param name="Var1">`
// declarations of the same name must dispatch event1 with both key/value
// pairs preserved. The W3C corpus marks this as a manual test because
// the SCXMLEventProcessor does not specify the wire format, so the
// per-binding observation lives in `<log expr="_event.raw">` for human
// inspection. The automated success criterion mirrors cpp `Test178.h` —
// reach the `final` state (any other arrival routes to the wildcard
// `fail` final). The duplicate-param emit shape is exercised at codegen
// time by the params loop in the bare-external send arm; the event
// dispatches verbatim and the receiving transition fires.
//
// W3C SCXML 6.2 (test178): `<send event="event1">` with two `<param
// name="Var1">` of the same name must dispatch verbatim — the params loop in
// the bare-external send arm walks both `<param>` declarations in document
// order, the receiving `event="event1"` transition fires before the wildcard
// `*` could route to fail, and the `<log expr="_event.raw">` action is
// observable side-effect only (test459 already pins the
// lua_eval_eventexpr-as-stringify path).
//
// W3C SCXML 6.2 + 6.4 + 5.3.3 manual-test automation lift — cpp `Test178.h` /
// `Test307.h` / `ScheduledAotTest`-shaped fixtures whose human-inspection
// contract collapses to "reach the `final` state, not `fail`". The W3C corpus
// marks these manual because the per-property observation (param key
// duplication, late-binding access timing, autoforward field equality,
// cancel-path onexit firing) lives in `<log>` output that no harness inspects
// automatically. cpp's runners already share this reduction — reaching the
// `final` state proves the upstream machinery (param emit, late binding,
// autoforward, cancel-path destroy) executed without diverting the SM into
// `fail` — so c11 mirrors the same single-bit verdict via the shared
// `_in_state(.., FINAL)` runner shape. No new template surface — these
// fixtures generate against the existing arms.

#include <stdio.h>

#include "../require_started.h"
#include "test178_sm.h"

int main(void) {
    test178_t sm;
    REQUIRE_STARTED(test178_init(&sm), TEST178_START_OK);
    test178_run(&sm);

    int rc = test178_ended_in(&sm, TEST178_STATE_FINAL) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test178: FAIL — active = 0x%08x\n", (unsigned)test178_active_states(&sm));
    }
    test178_destroy(&sm);
    return rc;
}
