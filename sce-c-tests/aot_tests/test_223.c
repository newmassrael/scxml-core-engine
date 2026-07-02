// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test223 — C11 AOT runner.
//
// Fixture (resources/223/test223.txml):
//   <data id="Var1"/>
//   <state id="s0">
//     <invoke type="http://www.w3.org/TR/scxml/" idlocation="Var1">
//       <content>...inline child reaching final immediately...</content>
//     </invoke>
//     <transition event="*" target="s1"/>
//   </state>
//   <state id="s1">
//     <transition cond="typeof Var1 !== 'undefined'" target="pass"/>
//     <transition target="fail"/>
//   </state>
//
// W3C 6.4.1 idlocation: the spec says when no `id` is supplied, the
// platform generates one and stores it in the variable named by
// `idlocation` so the parent can refer to the spawned child. The
// macrostep-end `execute_pending_invokes` spawns the child (initial
// is final → done.invoke synthesised); the parent's wildcard
// `event="*"` transition catches that done.invoke and moves to s1.
// The cond `typeof Var1 !== 'undefined'` then verifies that the
// auto-generated id was deposited at the spawn point — if the
// idlocation lower were a silent-broken hook, Var1 would still be
// Lua nil at that point, the cond would evaluate false, and the
// default arm would route to fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test223_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test223_t sm;
    test223_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test223_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test223: TIMEOUT — active = 0x%08x\n", (unsigned)test223_active_states(&sm));
            test223_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test223_tick(&sm);
    }

    int rc = test223_in_state(&sm, TEST223_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test223: FAIL — active = 0x%08x\n", (unsigned)test223_active_states(&sm));
    }
    test223_destroy(&sm);
    return rc;
}
