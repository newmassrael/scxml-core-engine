// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test364 — C11 AOT runner.
//
// W3C SCXML 3.6/3.13: exercises three forms of default-initial entry
// stacked in sequence — `initial="s11p112 s11p122"` (compound state
// attribute, multi-target), then `<initial><transition target="s21p112
// s21p122"/></initial>` (initial element), then `<state id="s3">` with
// no initial attribute (default to first child). Each leg requires the
// chain loop to enter every parallel sibling (s11p11+s11p12 via
// enter_parallel_regions, then the same for s2p1's regions). Reaching
// pass via the s3 → s31 → s311 → s3111 default-initial cascade proves
// every default-initial form along the way activated the full
// configuration; any single-leaf entry on the multi-target legs would
// trip a sibling's `<transition target="fail"/>` instead.
//
// Spec-mirror parity (cpp tests/CMakeLists.txt:796 registers the same
// fixture as `sce_generate_static_w3c_test(364)`).

#include <stdio.h>

#include "test364_sm.h"

int main(void) {
    test364_t sm;
    test364_init(&sm);
    test364_run(&sm);

    int rc = test364_in_state(&sm, TEST364_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test364: FAIL — active = 0x%08x\n", (unsigned)test364_active_states(&sm));
    }
    test364_destroy(&sm);
    return rc;
}
