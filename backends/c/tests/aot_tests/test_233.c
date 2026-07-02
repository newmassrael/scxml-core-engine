// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test233 — C11 AOT runner.
//
// Fixture (resources/233/test233.txml):
//   <datamodel><data id="Var1" expr="1"/></datamodel>
//   <state id="s0">
//     <onentry><send event="timeout" delay="3s"/></onentry>
//     <invoke type="http://www.w3.org/TR/scxml/">
//       <content><scxml initial="subFinal">
//         <final id="subFinal">
//           <onentry>
//             <send target="#_parent" event="childToParent">
//               <param name="aParam" expr="2"/>
//             </send>
//           </onentry>
//         </final>
//       </scxml></content>
//       <finalize>
//         <assign location="Var1" expr="_event.data.aParam"/>
//       </finalize>
//     </invoke>
//     <transition event="childToParent" cond="Var1 == 2" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C SCXML 6.4 — `<finalize>` runs BEFORE transition selection so
// finalize-assigned parent variables are visible to the cond. The child
// ships its `<param aParam=2/>` value to the parent via the new
// parent_dispatch `event_data` arg (a Lua-source table literal
// `{aParam = 2}` built in the child's lua_State by the new send-to-parent
// + params codegen branch). Parent's external dequeue hydrates
// `_event.data` from the string, runs the matching invoke's finalize
// body in parent's lua_State (transpiled JS → Lua via `to_lua_script`),
// which assigns `Var1 = _event.data.aParam` → Var1=2. The transition
// cond `Var1 == 2` then matches → s0 → pass.
//
// Failure modes: (a) without the cross-SM data carry the child's
// `<param>` value would never reach parent; `_event.data.aParam` would
// be nil; finalize would set Var1=nil (or fail silently); cond would
// be false; `event="*"` arm routes to fail. (b) without finalize
// execution Var1 stays 1; cond is false; same fail outcome. (c) if
// finalize ran AFTER transition selection the cond would see Var1=1
// (pre-finalize); same fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test233_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test233_t sm;
    test233_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test233_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test233: TIMEOUT — active = 0x%08x\n", (unsigned)test233_active_states(&sm));
            test233_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test233_tick(&sm);
    }

    int rc = test233_in_state(&sm, TEST233_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test233: FAIL — active = 0x%08x\n", (unsigned)test233_active_states(&sm));
    }
    test233_destroy(&sm);
    return rc;
}
