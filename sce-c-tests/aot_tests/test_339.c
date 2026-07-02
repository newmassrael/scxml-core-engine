// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test339 — C11 AOT runner.
//
// W3C SCXML 5.10.1: `_event.invokeid` is unset for events not produced by an `<invoke>` — the existing
// set_current_event lua chunk simply does not bind invokeid (carve-out tracked in c11_design_decisions.md until first
// invoke fixture lifts it), so internal raises leave it nil and the cond `typeof _event.invokeid === 'undefined'`
// matches pass.

#include <stdio.h>

#include "test339_sm.h"

int main(void) {
    test339_t sm;
    test339_init(&sm);
    test339_run(&sm);

    int rc = test339_in_state(&sm, TEST339_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test339: FAIL — active = 0x%08x\n", (unsigned)test339_active_states(&sm));
    }
    test339_destroy(&sm);
    return rc;
}
