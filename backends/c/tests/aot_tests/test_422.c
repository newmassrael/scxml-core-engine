// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test422 — C11 AOT runner.
//
// Fixture (resources/422/test422.txml):
//   <datamodel><data id="Var1" expr="0"/></datamodel>
//   <state id="s1" initial="s11">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <transition event="invokeS1 invokeS12">
//       <assign location="Var1" expr="Var1 + 1"/>
//     </transition>
//     <transition event="invokeS11" target="fail"/>
//     <transition event="timeout" cond="Var1 == 2" target="pass"/>
//     <transition event="timeout" target="fail"/>
//     <invoke><content><scxml initial="sub0"><state id="sub0">
//       <onentry><send target="#_parent" event="invokeS1"/></onentry>
//       <transition target="subFinal0"/>
//     </state><final id="subFinal0"/></scxml></content></invoke>
//     <state id="s11">
//       <invoke><content><scxml initial="sub1"><state id="sub1">
//         <onentry><send target="#_parent" event="invokeS11"/></onentry>
//         <transition target="subFinal1"/>
//       </state><final id="subFinal1"/></scxml></content></invoke>
//       <transition target="s12"/>
//     </state>
//     <state id="s12">
//       <invoke><content><scxml initial="sub2"><state id="sub2">
//         <onentry><send target="#_parent" event="invokeS12"/></onentry>
//         <transition target="subFinal2"/>
//       </state><final id="subFinal2"/></scxml></content></invoke>
//     </state>
//   </state>
//
// W3C SCXML 6.4: invokes are deferred to macrostep end (executePending
// hook) and cancelled by `onexit` of the deferring state. test422
// forces the cancel-during-step path:
//   - Init enters s1 → s11; both s1's and s11's invokes are pushed to
//     `pending_invokes` (state=s1, state=s11).
//   - Eventless transition s11→s12 fires inside `_init`'s macrostep
//     loop BEFORE `execute_pending_invokes` runs. s11's onexit hook
//     calls `sce_invoke_pending_cancel_for_state(state=s11)`, dropping
//     s11's entry from the queue. s12's onentry pushes its own invoke.
//   - `execute_pending_invokes` then processes s1 + s12 only — s11's
//     never spawns. Children's onentry sends `invokeS1` and
//     `invokeS12` to parent; parent's transition increments Var1
//     twice. After 2s `timeout` fires with Var1==2 → pass.
//
// If the cancel hook were missed (s11's invoke spawned anyway), the
// child's `invokeS11` would route parent → fail. The fixture pins
// per-state invoke cancellation timing across the eventless-step
// boundary that test226/240/241 don't exercise — those have only one
// invoke per state, so the cancel-on-eventless-exit path is dead code
// until this fixture forces it.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test422_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test422_t sm;
    test422_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test422_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test422: TIMEOUT — active = 0x%08x\n", (unsigned)test422_active_states(&sm));
            test422_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test422_tick(&sm);
    }

    int rc = test422_in_state(&sm, TEST422_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test422: FAIL — active = 0x%08x\n", (unsigned)test422_active_states(&sm));
    }
    test422_destroy(&sm);
    return rc;
}
