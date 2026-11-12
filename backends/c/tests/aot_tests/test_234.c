// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test234 — C11 AOT runner.
//
// Fixture (resources/234/test234.txml):
//   <datamodel><data id="Var1" expr="1"/><data id="Var2" expr="1"/></datamodel>
//   <parallel id="p0">
//     <onentry><send event="timeout" delay="3s"/></onentry>
//     <transition event="timeout" target="fail"/>
//     <state id="p01">
//       <invoke type="http://www.w3.org/TR/scxml/">
//         <content><scxml initial="subFinal1">
//           <final id="subFinal1">
//             <onentry><send target="#_parent" event="childToParent">
//               <param name="aParam" expr="2"/>
//             </send></onentry>
//           </final>
//         </scxml></content>
//         <finalize><assign location="Var1" expr="_event.data.aParam"/></finalize>
//       </invoke>
//       <transition event="childToParent" cond="Var1 == 2" target="s1"/>
//       <transition event="childToParent" target="fail"/>
//     </state>
//     <state id="p02">
//       <invoke type="http://www.w3.org/TR/scxml/">
//         <content><scxml initial="sub0">
//           <state id="sub0">
//             <onentry><send event="timeout" delay="2s"/></onentry>
//             <transition event="timeout" target="subFinal2"/>
//           </state>
//           <final id="subFinal2"/>
//         </scxml></content>
//         <finalize><assign location="Var2" expr="_event.data.aParam"/></finalize>
//       </invoke>
//     </state>
//   </parallel>
//   <state id="s1">
//     <transition cond="Var2 == 1" target="pass"/>
//     <transition target="fail"/>
//   </state>
//
// W3C SCXML 6.4 — per-invoke `<finalize>` runs ONLY for events whose
// `evt.invoke_id` matches that invoke's spawn-time id. p01's child
// raises `childToParent` (with p01's invoke_id stamped by
// `_init_with_parent`); the parent's external-dequeue finalize-arm
// (lifted in commit H, test233) iterates registered finalize bodies
// and runs only the one whose arm's `child_X_invoke_id` matches the
// dequeued event's `invoke_id`. p02's child never sends to parent, so
// p02's finalize never receives a matching event — Var2 stays 1.
//
// Parent's `<transition event="childToParent" cond="Var1 == 2"/>`
// matches AFTER p01's finalize runs (Var1=2) → transition to s1 (which
// exits p0, destroying both invokes). At s1 the eventless transition
// `cond="Var2 == 1"` matches (Var2 untouched) → pass.
//
// Failure modes: (a) if both finalises ran on every event Var2 would
// have been overwritten to 2 (the same `_event.data.aParam` p01 sees)
// and the s1 cond would fail → fail. (b) if p01's finalize ran AFTER
// transition selection Var1 would still be 1 → first cond fails →
// matches the unconditional `<transition target="fail"/>`.
//
// W3C SCXML 6.4 (test234): parallel parent (p0 with p01+p02 regions) each
// carrying its own `<invoke>` with its own `<finalize>`. p01's child has
// `<final>` initial that sends `<send target=#_parent><param aParam=2/>`;
// p02's child has a 2 s `<send event="timeout">` (intra-child, not to parent)
// so p02 never raises a `<send target=#_parent>` event. The test233
// per-invoke_id finalize-arm matches only the p01 invoke when `childToParent`
// arrives → p01 finalize sets Var1=2, p02 finalize stays unrun → Var2=1.
// Parent's `cond="Var1==2"` transitions to s1, then s1's eventless
// `cond="Var2==1"` routes to pass. Pins per-invoke isolation: a sibling
// invoke's finalize must NOT run on another sibling's events.
//
// W3C SCXML 6.4 invoke send-to-parent + finalize — `<send target="#_parent"
// event="X"><param name="K" expr="V"/></send>` in the child must carry param
// values to the parent's `_event.data.K`, AND each parent invoke's
// `<finalize>` body must run BEFORE transition selection so finalize-assigned
// parent vars are visible to the cond. cpp `StateMachine::processEvent` runs
// finalize via `InvokeExecutor::executeFinalize(invokeid)`; c11 mirrors via
// (a) parent_dispatch signature extended with `event_data` arg, (b) child's
// `<send target=#_parent>` with params codegen branch that builds a
// Lua-source table literal from each param expr (per-value formatter quotes
// strings, serialises numbers/booleans verbatim), (c) parent's
// `dispatch_external_by_name` shim copies `event_data` into
// `event_with_meta_t.data`, (d) parent's external dequeue runs `_event.data =
// (<table>)` then iterates registered finalize bodies, executing only those
// whose owning invoke's id matches `evt.invoke_id`. Per-invoke isolation pins
// test234's parallel-finalize fixture (each branch's finalize runs only for
// events from its own child).

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test234_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test234_t sm;
    REQUIRE_STARTED(test234_init(&sm), TEST234_START_OK);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test234_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test234: TIMEOUT — active = 0x%08x\n", (unsigned)test234_active_states(&sm));
            test234_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test234_tick(&sm);
    }

    int rc = test234_ended_in(&sm, TEST234_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test234: FAIL — active = 0x%08x\n", (unsigned)test234_active_states(&sm));
    }
    test234_destroy(&sm);
    return rc;
}
