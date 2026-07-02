// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test220 — C11 AOT runner.
//
// Fixture (resources/220/test220.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="5s"/></onentry>
//     <invoke type="http://www.w3.org/TR/scxml/">
//       <content>
//         <scxml initial="subFinal"><final id="subFinal"/></scxml>
//       </content>
//     </invoke>
//     <transition event="done.invoke" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.4 type URI form: the invoke's `type` attribute is the SCXML
// session URI rather than the bare `scxml` shorthand. The parser's
// invoke type-keyword normalisation accepts both equivalent strings,
// so the spawn path is identical to test247 — child reaches its
// top-level <final> synchronously inside `_init`, parent's
// `execute_pending_invokes` raises the generic `done.invoke` (no
// `done.invoke.<id>` transition in the parent → use_specific_event
// stays false) and s0→pass fires before the 5 s safety-net timeout.
// The wildcard `event="*"` arm is the failure route so any non-
// done.invoke event would route to fail; success means the runner
// observes pass without that arm firing.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test220_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test220_t sm;
    test220_init(&sm);

    const uint64_t timeout_ms = 8000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test220_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test220: TIMEOUT — active = 0x%08x\n", (unsigned)test220_active_states(&sm));
            test220_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test220_tick(&sm);
    }

    int rc = test220_in_state(&sm, TEST220_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test220: FAIL — active = 0x%08x\n", (unsigned)test220_active_states(&sm));
    }
    test220_destroy(&sm);
    return rc;
}
