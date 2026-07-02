// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test242 — C11 AOT runner.
//
// Fixture (resources/242/test242.txml):
//   <state id="s0">
//     <onentry><send event="timeout1" delay="1s"/></onentry>
//     <transition event="timeout" target="fail"/>
//     <invoke src="file:test242sub1.scxml"/>
//     <transition event="done.invoke" target="s02"/>
//     <transition event="timeout1" target="s03"/>
//   </state>
//   <state id="s02">
//     <onentry><send event="timeout2" delay="1s"/></onentry>
//     <invoke><content><scxml initial="subFinal1"><final id="subFinal1"/></scxml></content></invoke>
//     <transition event="done.invoke" target="pass"/>
//     <transition event="timeout2" target="fail"/>
//   </state>
//   <state id="s03">
//     <onentry><send event="timeout3" delay="1s"/></onentry>
//     <invoke><content><scxml initial="subFinal2"><final id="subFinal2"/></scxml></content></invoke>
//     <transition event="timeout3" target="pass"/>
//     <transition event="done.invoke" target="fail"/>
//   </state>
//
// W3C SCXML 6.4 — markup parity between `src=` (external file) and inline
// `<content>` for invoked services of the SCXML processor type. Three
// children share the same shape (initial state IS a top-level `<final>`
// so `done.invoke` fires synchronously during `execute_pending_invokes`):
// test242sub1.scxml is author-supplied (cmake `test${N}sub*.txml` glob),
// the two `<content>` blocks are parser-extracted via the §9.6.6
// synth-invoke rule into `test242__sce_synth_invoke__invoke_{1,2}.scxml`
// (cmake `test${N}__sce_synth_invoke__*.scxml` glob). All three feed the
// same downstream codegen path.
//
// Because each spawn raises `done.invoke` inside the same macrostep, s0
// transitions to s02 before timeout1 fires; s02 transitions to pass
// before timeout2 fires. The s03 path (timeout1-route) is not exercised
// because the src= invoke succeeds, but its presence in the fixture
// forces both s02 and s03 to register an inline-content child — the
// fixture pins parser handling of multiple `<content>` siblings within
// the same parent SM.
//
// Failure modes: if the src= path failed to spawn the s0 invoke, s0
// would fall through to s03 on timeout1 and the s03 inline-content
// invoke would fire done.invoke → fail. If only the inline-content
// path worked, s02's `done.invoke → pass` would never arrive and
// timeout2 would route to fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test242_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test242_t sm;
    test242_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test242_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test242: TIMEOUT — active = 0x%08x\n", (unsigned)test242_active_states(&sm));
            test242_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test242_tick(&sm);
    }

    int rc = test242_in_state(&sm, TEST242_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test242: FAIL — active = 0x%08x\n", (unsigned)test242_active_states(&sm));
    }
    test242_destroy(&sm);
    return rc;
}
