// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test226 — C11 AOT runner.
//
// Fixture (resources/226/test226.txml):
//   <state id="s0">
//     <onentry><send event="timeout" delay="3s"/></onentry>
//     <invoke type="..." src="file:test226sub1.scxml">
//       <param name="Var1" expr="1"/>
//     </invoke>
//     <transition event="varBound" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C 6.4.1 <param> value passing: the parent's `<param>` evaluates
// "1" in the parent's lua_State and the value is transferred onto the
// child's lua_State as Var1, overriding the child's default `<data
// id="Var1"/>` (unbound) seed. The child's eventless `conf:isBound="1"`
// transition then sees Var1=1, fires its `<send target="#_parent"
// event="varBound"/>` body, and reaches the child's <final>.
//
// The parent's `varBound` transition catches the dispatch via
// `parent_dispatch` → `dispatch_external_by_name` and routes to pass.
// If the param transfer were a silent-broken hook, Var1 would stay
// nil, the default transition would fire, and no varBound would
// reach the parent → 3s timeout fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test226_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test226_t sm;
    test226_init(&sm);

    const uint64_t timeout_ms = 6000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test226_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test226: TIMEOUT — active = 0x%08x\n",
                    (unsigned)test226_active_states(&sm));
            test226_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test226_tick(&sm);
    }

    int rc = test226_in_state(&sm, TEST226_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test226: FAIL — active = 0x%08x\n",
                (unsigned)test226_active_states(&sm));
    }
    test226_destroy(&sm);
    return rc;
}
