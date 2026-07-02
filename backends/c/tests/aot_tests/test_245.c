// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test245 — C11 AOT runner.
//
// Fixture (resources/245/test245.txml):
//   <data id="Var2" expr="3"/>
//   <state id="s0">
//     <onentry><send event="timeout" delay="2s"/></onentry>
//     <invoke ... namelist="Var2">
//       <content>...child with NO <data id="Var2"/>...
//         conf:isBound="2" → failure
//         else → success
//       </content>
//     </invoke>
//     <transition event="success" target="pass"/>
//     <transition event="*" target="fail"/>
//
// W3C 6.4.1 namelist undeclared-variable filter: parent's Var2=3
// IS in the parent datamodel, but the child has no `<data id="Var2"/>`.
// Per W3C 6.4 (and cpp's `DatamodelValidationHelper::isVariableDeclaredInChild`
// filter), the namelist push must be skipped — the child's Var2 stays
// unbound (Lua nil). Child's `conf:isBound="2"` cond therefore evaluates
// false, the default transition fires → `success` → parent reaches pass.
// If the codegen-time filter (child_datamodel_vars membership check)
// were missing, Var2 would be set to 3 in the child, isBound=true, and
// the failure path would reach the parent → fail.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test245_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test245_t sm;
    test245_init(&sm);

    const uint64_t timeout_ms = 5000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test245_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test245: TIMEOUT — active = 0x%08x\n", (unsigned)test245_active_states(&sm));
            test245_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test245_tick(&sm);
    }

    int rc = test245_in_state(&sm, TEST245_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test245: FAIL — active = 0x%08x\n", (unsigned)test245_active_states(&sm));
    }
    test245_destroy(&sm);
    return rc;
}
