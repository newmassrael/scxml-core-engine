// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test504 — C11 AOT runner.
//
// W3C SCXML 3.13: external transition exits all states up to the LCCA — the s211→s2 transition's exit set must include
// s211, s21, s2 (not just s211), so the receiving s2-level catch confirms only one onexit per ancestor before the entry
// chain rebuilds. Tests find_lcca + compute_exit_set's external-self-loop classification.
//
// W3C SCXML 3.13: external transition exits all states up to the LCCA — the
// s2-level external transition's exit set includes the entire ancestor chain;
// exercises find_lcca + compute_exit_set (from 266dc8d8).

#include <stdio.h>

#include "../require_started.h"
#include "test504_sm.h"

int main(void) {
    test504_t sm;
    REQUIRE_STARTED(test504_init(&sm), TEST504_START_OK);
    test504_run(&sm);

    int rc = test504_ended_in(&sm, TEST504_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test504: FAIL — active = 0x%08x\n", (unsigned)test504_active_states(&sm));
    }
    test504_destroy(&sm);
    return rc;
}
