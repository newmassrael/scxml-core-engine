// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test504 — C11 AOT runner.
//
// W3C SCXML 3.13: external transition exits all states up to the LCCA — the s211→s2 transition's exit set must include
// s211, s21, s2 (not just s211), so the receiving s2-level catch confirms only one onexit per ancestor before the entry
// chain rebuilds. Tests find_lcca + compute_exit_set's external-self-loop classification (옵션 ξ landed).

#include <stdio.h>

#include "test504_sm.h"

int main(void) {
    test504_t sm;
    test504_init(&sm);
    test504_run(&sm);

    int rc = test504_in_state(&sm, TEST504_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test504: FAIL — active = 0x%08x\n", (unsigned)test504_active_states(&sm));
    }
    test504_destroy(&sm);
    return rc;
}
