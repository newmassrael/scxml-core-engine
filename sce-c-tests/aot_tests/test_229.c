// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test229 — C11 AOT runner.
//
// Fixture (resources/229/test229.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="3s"/></onentry>
//     <invoke type="http://www.w3.org/TR/scxml/" autoforward="true">
//       <content><scxml initial="sub0">
//         <state id="sub0">
//           <onentry>
//             <send target="#_parent" event="childToParent"/>
//             <send event="timeout" delay="3s"/>
//           </onentry>
//           <transition event="childToParent" target="subFinal">
//             <send target="#_parent" event="eventReceived"/>
//           </transition>
//           <transition event="*" target="subFinal"/>
//         </state>
//         <final id="subFinal"/>
//       </scxml></content>
//     </invoke>
//     <transition event="childToParent"/>
//     <transition event="eventReceived" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C SCXML 6.4 — `autoforward="true"` requires the parent to ship an
// exact copy of every external event it receives to the active child.
// Child's onentry sends `childToParent` to `#_parent`; parent receives
// it on the external queue, the new `forward_to_autoforward_children`
// helper (state_machine.c.jinja2, gated on `model.has_autoforward_invoke`)
// pushes the same name onto the child's external queue via the child's
// `_raise_external_by_name` shim. Parent's no-target
// `<transition event="childToParent"/>` consumes the parent-side copy
// (no configuration change). The post-step `_drive_active_children`
// then runs the child's `_step` which dequeues the forwarded
// `childToParent`, fires the eventless transition to `subFinal` with
// `<send target="#_parent" event="eventReceived"/>`. Parent's next
// `_step` pops `eventReceived` from its external queue → s0 → pass.
//
// Forwarder filters: platform events (done.*, error.*, cancel.invoke)
// are NOT forwarded, mirroring cpp `StateMachine::processEvent`'s
// `isPlatformEvent` guard. Child SMs whose enum doesn't contain the
// forwarded name silently fall through inside their shim — no link or
// runtime error.
//
// Failure modes: if the forwarder were absent the child would never
// receive `childToParent` back; the child's 3 s `timeout` would fire
// (intra-child) → `<transition event="*"/>` to `subFinal` → child
// raises `done.invoke` to parent → parent's `<transition event="*"/>`
// matches → fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test229_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test229_t sm;
    test229_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test229_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test229: TIMEOUT — active = 0x%08x\n", (unsigned)test229_active_states(&sm));
            test229_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test229_tick(&sm);
    }

    int rc = test229_in_state(&sm, TEST229_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test229: FAIL — active = 0x%08x\n", (unsigned)test229_active_states(&sm));
    }
    test229_destroy(&sm);
    return rc;
}
