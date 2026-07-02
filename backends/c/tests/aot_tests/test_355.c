// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test355 — C11 AOT runner.
//
// W3C SCXML 3.6: default initial = first child in document order.
// test355 has four states (s0, s1, pass, fail); s0 is the document-order
// first state, so the engine must start in s0. The eventless transition
// out of s0 targets `pass`, and the eventless transition out of s1
// targets `fail` — so a working engine reaches `pass`, a buggy one that
// initialised on `s1` would reach `fail` instead.
//
// Exit status is the verdict (0 = PASS, 1 = FAIL); CTest reports it as
// the test result. No GTest because the C11 backend has no C++ runtime
// link — keeping the runner C11-pure preserves the cross-compile story
// for the watching-zenoh MCU target.

#include <stdio.h>

#include "test355_sm.h"

int main(void) {
    test355_t sm;
    test355_init(&sm);
    test355_run(&sm);

    if (test355_in_state(&sm, TEST355_STATE_PASS)) {
        return 0;
    }
    fprintf(stderr, "test355: FAIL — active = 0x%08x\n", (unsigned)test355_active_states(&sm));
    return 1;
}
