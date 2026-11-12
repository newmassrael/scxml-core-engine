// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test554 — C11 AOT runner.
//
// Fixture (resources/554/test554.txml):
//   <state id="s0">
//     <onentry><send event="timer" delay="1s"/></onentry>
//     <invoke type="http://www.w3.org/TR/scxml/"
//             namelist="__undefined_variable_for_error__">
//       <content><scxml initial="subFinal"><final id="subFinal"/></scxml></content>
//     </invoke>
//     <transition event="timer" target="pass"/>
//     <transition event="done.invoke" target="fail"/>
//   </state>
//
// W3C SCXML 6.4 — `<invoke namelist>` must terminate processing of the
// element if any name in the list is not declared in the parent's
// datamodel. The `__undefined_variable_for_error__` name is not in any
// `<data>` element of test554.scxml, so codegen statically detects the
// undeclared reference (membership check against `model.variables`) and
// emits an `error.execution` raise + `continue` in the spawn block,
// skipping the child SM `_init` call. The child never runs, never
// raises `done.invoke`, and the parent's 1 s `timer` fires unopposed →
// s0 → pass. cpp `InvokeExecutor::startInvoke`
// (sce/src/runtime/InvokeExecutor.cpp:425-432) does the same check at
// runtime via `hasVariable(parentSessionId, varName)` + `destroySession +
// return ""` — c11 folds the parent-side check to a compile-time
// predicate because parent vars are statically known.
//
// If the codegen-time check were missed, the child would spawn, reach
// `<final>` immediately during `execute_pending_invokes`, and `done.invoke`
// would route to fail.
//
// W3C SCXML 6.4.1 (test554): `<invoke
// namelist="__undefined_variable_for_error__">` references a name not in the
// parent's `<datamodel>`. Codegen-time predicate `loop.first` arm in
// `c/invoke_methods.jinja2` raises `EVENT_ERROR_EXECUTION` to the parent's
// internal queue and `continue`s past the spawn — child SM `_init` never
// runs, no `done.invoke` is raised. Parent's 1 s `timer` fires unopposed → s0
// → pass. If the static check were missed the child (a top-level `<final>`)
// would spawn during `execute_pending_invokes` and synthesise `done.invoke`
// immediately, routing parent → fail.
//
// W3C SCXML 6.4 invoke namelist validation — the `<invoke namelist="...">`
// attribute names must each be declared in the PARENT's `<datamodel>`; if any
// name is missing, the processor MUST terminate the invocation and raise
// error.execution (cpp `InvokeExecutor::startInvoke`
// sce/src/runtime/InvokeExecutor.cpp:425-432 calls `destroySession + return
// ""` at runtime). c11 folds the membership check to a compile-time
// predicate: the codegen iterates `invoke_info.namelist.split()` against
// `model.variables | map(id)` and emits the error.execution raise +
// `continue` in `execute_pending_invokes` exactly when a name is undeclared.
// The skip-spawn branch sits before the child SM `_init` call so no resources
// are allocated. Verified non-regressing against test245 (declared
// namelist=Var2) — its error.execution emission count is unchanged.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test554_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test554_t sm;
    REQUIRE_STARTED(test554_init(&sm), TEST554_START_OK);

    const uint64_t timeout_ms = 3000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test554_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test554: TIMEOUT — active = 0x%08x\n", (unsigned)test554_active_states(&sm));
            test554_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test554_tick(&sm);
    }

    int rc = test554_ended_in(&sm, TEST554_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test554: FAIL — active = 0x%08x\n", (unsigned)test554_active_states(&sm));
    }
    test554_destroy(&sm);
    return rc;
}
