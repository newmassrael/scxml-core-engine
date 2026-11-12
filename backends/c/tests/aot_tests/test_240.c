// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test240 — C11 AOT runner.
//
// Fixture (resources/240/test240.txml):
//   <data id="Var1" expr="1"/>
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke ... namelist="Var1"><content>...child with <data id="Var1" expr="0">...</content></invoke>
//     ...success → s02 with <param name="Var1" expr="1"/> ... → pass
//
// W3C 6.4.1 namelist + param: parent's Var1=1 transfers via namelist
// (s01 invoke) and via <param> (s02 invoke) to the child's Var1
// slot, overriding the child's default (Var1=0). Each child's
// eventless `Var1==1` cond fires, dispatching `success` to parent
// via #_parent → parent advances to next state until pass.
//
// W3C SCXML 6.4.1 (test240): two-arm fixture exercising namelist (s01) AND
// `<param>` (s02) — both mechanisms must transfer Var1=1 from parent to child
// for the test to reach pass. First arm pins namelist transfer parity with
// cpp; second arm pins `<param expr>` evaluation in the parent's lua_State.
//
// The lowering this fixture shares with related fixtures is described in
// test_226.c.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "../require_started.h"
#include "test240_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test240_t sm;
    REQUIRE_STARTED(test240_init(&sm), TEST240_START_OK);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test240_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test240: TIMEOUT — active = 0x%08x\n", (unsigned)test240_active_states(&sm));
            test240_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test240_tick(&sm);
    }

    int rc = test240_ended_in(&sm, TEST240_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test240: FAIL — active = 0x%08x\n", (unsigned)test240_active_states(&sm));
    }
    test240_destroy(&sm);
    return rc;
}
