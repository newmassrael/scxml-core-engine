// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test241 — C11 AOT runner.
//
// Fixture (resources/241/test241.txml):
//   <data id="Var1" expr="1"/>
//   <state id="s0" initial="s01">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <state id="s01"> invoke namelist="Var1" → success → s02 (failure → s03)
//     <state id="s02"> invoke <param name="Var1" expr="1"/> → success → pass (else fail)
//     <state id="s03"> invoke <param name="Var1" expr="1"/> → failure → pass (else fail)
//
// W3C 6.4.1 namelist + param parity: tests that namelist (s01) and
// <param> (s02/s03) both transfer Var1=1 from the parent's datamodel
// into the child's Var1 slot. With both transfers honoured, s01's
// child fires `success` → parent moves to s02 → that child also
// fires `success` → parent reaches pass.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test241_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test241_t sm;
    test241_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test241_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test241: TIMEOUT — active = 0x%08x\n", (unsigned)test241_active_states(&sm));
            test241_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test241_tick(&sm);
    }

    int rc = test241_in_state(&sm, TEST241_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test241: FAIL — active = 0x%08x\n", (unsigned)test241_active_states(&sm));
    }
    test241_destroy(&sm);
    return rc;
}
