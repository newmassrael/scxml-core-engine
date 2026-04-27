// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test576 — C11 AOT runner.
//
// W3C SCXML 3.6/3.13: same multi-target initial axis as test413, but
// the entry trace exercises raise/match across the parallel siblings
// rather than In() — s11p112's onentry raises `In-s11p112`, which
// s11p122 picks up via `<transition event="In-s11p112" target="pass"/>`.
// The transition fires only when both legs are simultaneously active
// (s11p122 has to be in the configuration to receive the event); a
// single-leaf entry (the pre-fix behavior) would land in s11p11's
// default first child, never enter s11p12, and the safety-net 1s
// `<send event="timeout"/>` would route to fail.
//
// Spec-mirror parity (test413 sibling — same enter_state_recursive
// chain expansion through the parallel chain element).

#include <stdio.h>

#include "test576_sm.h"

int main(void) {
    test576_t sm;
    test576_init(&sm);
    test576_run(&sm);

    int rc = test576_in_state(&sm, TEST576_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test576: FAIL — active = 0x%08x\n", (unsigned)test576_active_states(&sm));
    }
    test576_destroy(&sm);
    return rc;
}
