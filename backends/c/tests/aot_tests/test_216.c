// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test216_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test216_t sm;
    test216_init(&sm);

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

    int rc = test216_in_state(&sm, TEST216_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test216: FAIL — active = 0x%08x\n", (unsigned)test216_active_states(&sm));
    }
    test216_destroy(&sm);
    return rc;
}
