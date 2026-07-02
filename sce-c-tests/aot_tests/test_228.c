// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test228 — C11 AOT runner.
//
// Fixture (resources/228/test228.txml):
//   <data id="Var1"/>
//   <state id="s0">
//     <invoke type="http://www.w3.org/TR/scxml/" id="foo">
//       <content>...inline child reaching final immediately...</content>
//     </invoke>
//     <transition event="done.invoke" target="s1">
//       <assign location="Var1" expr="_event.invokeid"/>
//     </transition>
//     <transition event="*" target="fail"/>
//   </state>
//   <state id="s1">
//     <transition cond="Var1 == 'foo'" target="pass"/>
//     <transition target="fail"/>
//   </state>
//
// W3C 5.10.1 (.invokeid): every event raised by or originating from an
// invoked child carries the spawning invoke's id. Here the child's
// done.invoke arrives at the parent with `_event.invokeid == 'foo'`
// because the parent supplied an explicit `id="foo"` (which the codegen
// preserves verbatim into the `_invoke_id_buf`). The transition
// assigns `_event.invokeid` to Var1, then s1's cond verifies the id
// matches the literal — the round-trip pins the C-side `evt.invoke_id`
// → lua-side `_pending_event_invokeid` → `_event.invokeid` chain.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test228_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test228_t sm;
    test228_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test228_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test228: TIMEOUT — active = 0x%08x\n", (unsigned)test228_active_states(&sm));
            test228_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test228_tick(&sm);
    }

    int rc = test228_in_state(&sm, TEST228_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test228: FAIL — active = 0x%08x\n", (unsigned)test228_active_states(&sm));
    }
    test228_destroy(&sm);
    return rc;
}
