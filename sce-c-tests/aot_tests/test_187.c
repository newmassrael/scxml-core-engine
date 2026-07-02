// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test187 — C11 AOT runner.
//
// Fixture (resources/187/test187.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="1s"/></onentry>
//     <invoke type="scxml"><content>
//       <scxml initial="sub0">
//         <state id="sub0">
//           <onentry>
//             <send event="childToParent" target="#_parent" delay=".5"/>
//           </onentry>
//           <transition target="subFinal"/>
//         </state>
//         <final id="subFinal"/>
//       </scxml>
//     </content></invoke>
//     <transition event="childToParent" target="fail"/>
//     <transition event="timeout" target="pass"/>
//   </state>
//
// W3C SCXML 6.2 termination semantics: the child schedules a delayed
// send to its parent, then immediately takes an eventless transition
// to `subFinal` (a top-level `<final>`). The child's macrostep — driven
// by the parent's `execute_pending_invokes` 16-iteration loop — reaches
// final before any `_tick` call against it. Once `is_in_final_state`
// flips on, `_drive_active_children` skips the child and its
// `scheduled_queue` is never pumped again. The .5s delayed entry stays
// in the buffer but never promotes; parent's 1s `timeout` fires
// unopposed and routes s0→pass.
//
// If the codegen routed delayed `<send target="#_parent">` through
// `parent_dispatch` immediately (rather than scheduling), parent's
// `childToParent` transition would catch it and route to fail. If the
// generated code lacked the `to_parent` discriminator and tried to
// raise into the child's local enum, EVENT_NONE would land on the
// external queue (silent drop) and the test would still pass — but
// for the wrong reason; the dispatch shape mirrors cpp's per-child
// `parentSendScheduler_` so future async-final fixtures have a real
// path to fire `_tick`-promoted entries through `parent_dispatch`.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test187_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test187_t sm;
    test187_init(&sm);

    const uint64_t timeout_ms = 3000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test187_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test187: TIMEOUT — active = 0x%08x\n", (unsigned)test187_active_states(&sm));
            test187_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test187_tick(&sm);
    }

    int rc = test187_in_state(&sm, TEST187_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test187: FAIL — active = 0x%08x\n", (unsigned)test187_active_states(&sm));
    }
    test187_destroy(&sm);
    return rc;
}
