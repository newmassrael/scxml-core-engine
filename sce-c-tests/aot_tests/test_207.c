// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test207 — C11 AOT runner.
//
// Fixture (resources/207/test207.txml):
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke type="scxml"><content>
//       <scxml initial="sub0">
//         <state id="sub0">
//           <onentry>
//             <send event="event1" id="foo" delay="1s"/>
//             <send event="event2" delay="1.5s"/>
//             <send target="#_parent" event="childToParent"/>
//           </onentry>
//           <transition event="event1" target="subFinal"><send target="#_parent" event="pass"/></transition>
//           <transition event="*" target="subFinal"><send target="#_parent" event="fail"/></transition>
//         </state>
//         <final id="subFinal"/>
//       </scxml>
//     </content></invoke>
//     <state id="s01"><transition event="childToParent" target="s02"><cancel sendid="foo"/></transition></state>
//     <state id="s02">
//       <transition event="pass" target="pass"/>
//       <transition event="fail" target="fail"/>
//       <transition event="timeout" target="fail"/>
//     </state>
//   </state>
//
// Pins: parent's `<cancel sendid="foo">` runs in the parent's session
// and MUST NOT touch the child's scheduler queue. The child must keep
// ticking until its 1 s `event1` fires → child sends `pass` → parent
// reaches pass before its 2 s timeout. `_drive_active_children` calls
// the child's `_tick` (parsed from the child SCXML's `<send delay>`
// presence) so its scheduler promotes elapsed entries on the same
// outer poll iteration.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test207_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test207_t sm;
    test207_init(&sm);

    const uint64_t timeout_ms = 4000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test207_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test207: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test207_active_states(&sm));
            test207_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test207_tick(&sm);
    }

    int rc = test207_in_state(&sm, TEST207_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test207: FAIL — active = 0x%08x\n",
                (unsigned)test207_active_states(&sm));
    }
    test207_destroy(&sm);
    return rc;
}
