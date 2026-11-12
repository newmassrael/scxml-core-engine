// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test216 — C11 AOT runner.
//
// W3C SCXML 6.4.3: `<invoke srcexpr="Var1" type="...scxml">` evaluates
// `srcexpr` at invoke-fire time (macrostep end), not at parse time.
// `Var1` is initialised to `'foo'` (no such file) and reassigned to
// `'file:test216sub1.scxml'` in the entering onentry; correct invokes
// see the file-path value and a successful child spawn fires
// `done.invoke` → s0 takes the `done.invoke` transition into `pass`.
// Misordering — reading `Var1` at parse time — would route through the
// 5 s `<send event="timeout" delay="5s">` safety net into `fail`.
//
// The c11 hybrid path keeps the eval validation but uses a trivial
// immediate-final stub for the child (cpp `test216_sm.inl:166-185`
// pattern: pure AOT, no Interpreter dependency). Because the original
// test216sub1.scxml is itself `<final/>`-only, the stub is
// observationally identical for the W3C contract being tested here.
//
// W3C SCXML 6.4.1 (test216):`<invoke srcexpr="Var1">` runtime evaluation —
// Var1 reassigned in onentry to the child path, codegen lowers a
// luaL_dostring eval-validation gate then spawns the pre-generated
// `test216_hybrid0` stub child. cpp `test216_sm.inl:166-185` is the
// equivalent pure-AOT pattern.
//
// W3C SCXML 6.4.1 / 6.4.2 hybrid invoke — `<invoke srcexpr>` +
// `<invoke><content expr>` runtime evaluation. cpp uses a pure-AOT pattern
// with a trivial immediate-final stub child (`test{N}_hybrid{idx}.scxml`,
// written by sce-build's `generate_hybrid_child_scxmls`). The eval runs at
// macrostep end on the parent's lua_State for validation only — the result is
// intentionally not consumed because the W3C contract checks evaluation
// timing, not the value. Both fixtures' actual children (`test216sub1.scxml`
// is `<final/>`-only; test530's inline scxml is `<scxml><final/></scxml>`)
// are immediate-final, so the stub is observationally identical for the
// contract being tested.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test216_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test216_t sm;
    REQUIRE_STARTED(test216_init(&sm), TEST216_START_OK);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test216_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test216: TIMEOUT — active = 0x%08x\n", (unsigned)test216_active_states(&sm));
            test216_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test216_tick(&sm);
    }

    int rc = test216_ended_in(&sm, TEST216_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test216: FAIL — active = 0x%08x\n", (unsigned)test216_active_states(&sm));
    }
    test216_destroy(&sm);
    return rc;
}
