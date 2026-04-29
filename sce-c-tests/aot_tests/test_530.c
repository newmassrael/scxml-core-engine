// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test530 — C11 AOT runner.
//
// W3C SCXML 6.4.4: `<invoke type="...scxml"><content expr="Var1"/></invoke>`
// evaluates `<content>`'s `expr` at invoke-fire time (macrostep end),
// not at parse time. `Var1` is initialised to integer `1` and reassigned
// to a literal SCXML scriptlet in the entering onentry; correct invokes
// see the SCXML string at fire time and a successful child spawn fires
// `done.invoke` → s0 takes the `done.invoke` transition into `pass`.
// Misordering — reading `Var1` at parse time — would route through the
// 2 s `<send event="timeout" delay="2s">` safety net into `fail`.
//
// The c11 hybrid path keeps the eval validation but uses a trivial
// immediate-final stub for the child (cpp `test530_sm.inl:166-185`
// pattern: pure AOT, no Interpreter dependency). The original
// inline scxml is `<scxml version="1.0"><final/></scxml>` — itself an
// immediate-final machine — so the stub is observationally identical
// for the W3C contract being tested here.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test530_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test530_t sm;
    test530_init(&sm);

    const uint64_t timeout_ms = 3000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test530_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test530: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test530_active_states(&sm));
            test530_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test530_tick(&sm);
    }

    int rc = test530_in_state(&sm, TEST530_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test530: FAIL — active = 0x%08x\n",
                (unsigned)test530_active_states(&sm));
    }
    test530_destroy(&sm);
    return rc;
}
