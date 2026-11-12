// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test232 — C11 AOT runner.
//
// Fixture (resources/232/test232.txml):
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="3s"/></onentry>
//     <invoke><content>
//       <scxml initial="subFinal">
//         <final id="subFinal">
//           <onentry>
//             <send target="#_parent" event="childToParent1"/>
//             <send target="#_parent" event="childToParent2"/>
//           </onentry>
//         </final>
//       </scxml>
//     </content></invoke>
//     <transition event="timeout" target="fail"/>
//     <state id="s01"><transition event="childToParent1" target="s02"/></state>
//     <state id="s02"><transition event="childToParent2" target="s03"/></state>
//     <state id="s03"><transition event="done.invoke" target="pass"/></state>
//   </state>
//
// W3C SCXML 6.4 — multi-event delivery from a child. Child's initial
// is `subFinal` itself (a top-level `<final>` whose `<onentry>` carries
// two `<send target="#_parent">` blocks). The codegen's
// `_init_with_parent` stamps `parent_dispatch` BEFORE the configuration
// entry walk, so both sends route through `dispatch_external_by_name`
// onto the parent's external queue — the FIFO order is `childToParent1`
// then `childToParent2`. After init returns, `execute_pending_invokes`
// detects `is_in_final_state=true` and raises `done.invoke` (the third
// event in the parent's queue). Parent walks s01→s02→s03→pass.
//
// If the second `<send target="#_parent">` were silently dropped (e.g.
// the dispatch shim returned early after the first match), parent
// would stall in s02 and the 3 s timeout would route to fail.
//
// W3C SCXML 6.4 invoke — child shapes that the existing infrastructure
// already covers without new template surface. test232: child whose
// `initial=` IS a top-level `<final>` (final's `<onentry>` runs during the
// parent's `execute_pending_invokes` entry walk, so multiple `<send
// target="#_parent">` blocks all dispatch through the live `parent_dispatch`
// before `done.invoke`). test237: child has its own `<send delay>` scheduler
// but parent cancels via state-exit; the existing `destroy_active_children`
// onexit hook + child SM zero-init reclaims the in-flight scheduled entry.
// test239: dual-shape parity (one `src=file:` external child sibling + one
// inline `<content><scxml>` child) — the author-supplied sibling, which the
// parent's plan names and the build stages beside it, and the synth-invoke
// extraction converge on the same downstream codegen.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test232_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test232_t sm;
    REQUIRE_STARTED(test232_init(&sm), TEST232_START_OK);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test232_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test232: TIMEOUT — active = 0x%08x\n", (unsigned)test232_active_states(&sm));
            test232_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test232_tick(&sm);
    }

    int rc = test232_ended_in(&sm, TEST232_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test232: FAIL — active = 0x%08x\n", (unsigned)test232_active_states(&sm));
    }
    test232_destroy(&sm);
    return rc;
}
