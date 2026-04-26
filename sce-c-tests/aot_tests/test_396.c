// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test396 — C11 AOT runner.
//
// W3C SCXML 5.10 / 3.13: When two transitions list the same `event="foo"`,
// document order must select the first one (first-match-wins). test396
// raises `foo` from s0's onentry and declares `<transition event="foo"
// target="pass"/>` before `<transition event="foo" target="fail"/>` —
// reaching the second transition would prove the engine is reordering or
// double-firing.
//
// Fixture-only landing: the existing C11 transition emit walks the
// transitions list in document order and returns on the first matching
// arm (state_machine.c.jinja2 process_transition switch+if chain).
// needs_script_engine=false (no datamodel) so the MCU zero-deps profile
// is preserved — this fixture links without lua54.

#include <stdio.h>

#include "test396_sm.h"

int main(void) {
    test396_t sm;
    test396_init(&sm);
    test396_run(&sm);

    test396_state_t final = test396_get_current_state(&sm);
    int rc = (final == TEST396_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test396: FAIL — final state = %d\n", (int)final);
    }
    test396_destroy(&sm);
    return rc;
}
