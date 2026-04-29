// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test239 — C11 AOT runner.
//
// Fixture (resources/239/test239.txml):
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <transition event="timeout" target="fail"/>
//     <state id="s01">
//       <invoke src="file:test239sub1.scxml"/>
//       <transition event="done.invoke" target="s02"/>
//     </state>
//     <state id="s02">
//       <invoke><content><scxml initial="final"><final id="final"/></scxml></content></invoke>
//       <transition event="done.invoke" target="pass"/>
//     </state>
//   </state>
//
// W3C SCXML 6.4 — markup parity between `src=` (external file) and
// inline `<content>`. test239sub1.scxml is an author-supplied sibling
// (resources/239/test239sub1.scxml); cmake auto-discovery
// (`SCEStaticW3CTest.cmake` test${N}sub*.txml glob, landed alongside
// test226/276) stages it as a separate child SM source. Both invokes
// converge on the same shape: child's initial state IS a top-level
// `<final>`, so each spawn raises `done.invoke` immediately during
// `execute_pending_invokes`, walking parent s01→s02→pass before the
// 2 s timeout fires.
//
// If the `src=` path failed to resolve (e.g. parser dropping the
// invoke for unknown source), s01 would stall on done.invoke, the 2 s
// timeout would route to fail. If only one of the two invokes worked,
// the test would fail — the fixture pins parity between both shapes.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test239_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test239_t sm;
    test239_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test239_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test239: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test239_active_states(&sm));
            test239_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test239_tick(&sm);
    }

    int rc = test239_in_state(&sm, TEST239_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test239: FAIL — active = 0x%08x\n",
                (unsigned)test239_active_states(&sm));
    }
    test239_destroy(&sm);
    return rc;
}
