// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test215 — C11 AOT runner.
//
// Fixture (resources/215/test215.txml):
//   <datamodel><data id="Var1" expr="'foo'"/></datamodel>
//   <state id="s0">
//     <onentry>
//       <send event="timeout" delay="5s"/>
//       <assign location="Var1" expr="'http://www.w3.org/TR/scxml/'"/>
//     </onentry>
//     <invoke typeexpr="Var1">
//       <content><scxml initial="subFinal"><final id="subFinal"/></scxml></content>
//     </invoke>
//     <transition event="done.invoke" target="pass"/>
//     <transition event="*" target="fail"/>
//   </state>
//
// W3C SCXML 6.4 invoke + typeexpr: the spec requires runtime evaluation
// of typeexpr against the invoke's actual platform type. The C11 AOT
// codegen — like cpp AOT — folds typeexpr-only invokes whose `type=""`
// and inline `<content><scxml>` is present into the static-invoke
// shape: the child is spawned unconditionally, mirroring cpp's
// `is_static_invoke = type.empty() + has_static_child + !srcexpr +
// !contentexpr`. Because the parent's onentry assigns Var1 to the
// SCXML processor URI BEFORE the invoke is scheduled, both shapes
// (eval-typeexpr or skip-typeexpr) converge on the same observable —
// the spawn succeeds, the child reaches subFinal immediately, and
// done.invoke routes parent → pass.
//
// If the codegen treated typeexpr at parse time using Var1's initial
// `'foo'` value, the invoke would never spawn (foo is not a supported
// SCXML processor URI) and the parent's 5s timeout would fire → fail.
// The fixture pins that the codegen does NOT cement the parse-time
// value of Var1 into the static decision — it relies on the invoke
// running in the parent's macrostep AFTER onentry's assign.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "test215_sm.h"

extern uint64_t _sce_clock_now_ms(void);

int main(void) {
    test215_t sm;
    test215_init(&sm);

    const uint64_t timeout_ms = 7000u;
    const struct timespec poll_ts = {0, 10L * 1000L * 1000L};
    const uint64_t start_ms = _sce_clock_now_ms();

    while (!test215_is_in_final_state(&sm)) {
        if (_sce_clock_now_ms() - start_ms > timeout_ms) {
            fprintf(stderr, "test215: TIMEOUT — active = 0x%08x\n", (unsigned)test215_active_states(&sm));
            test215_destroy(&sm);
            return 1;
        }
        nanosleep(&poll_ts, NULL);
        test215_tick(&sm);
    }

    int rc = test215_in_state(&sm, TEST215_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test215: FAIL — active = 0x%08x\n", (unsigned)test215_active_states(&sm));
    }
    test215_destroy(&sm);
    return rc;
}
