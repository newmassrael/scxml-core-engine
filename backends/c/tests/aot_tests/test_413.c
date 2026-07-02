// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test413 — C11 AOT runner.
//
// W3C SCXML 3.6/3.13: <scxml initial="s2p112 s2p122"> declares a
// space-separated multi-target initial that bypasses the default
// initial path and enters two siblings of the same parallel state.
// parser.rs apply_parallel_initial_overrides walks each target's
// ancestor chain and rewrites every compound ancestor's `initial` to
// the path child (parallel parents are skipped — they enter every
// region anyway). The C11 enter_state_recursive's chain loop expands
// each parallel chain element via enter_parallel_regions, so when the
// chain passes through s2p1 every region (s2p11, s2p12) enters and
// follows the override-rewritten initial down to s2p112 / s2p122
// respectively. The pass transitions cross-check via In() —
// `<transition cond="In('s2p122')" target="pass"/>` in s2p112 fires
// only when both leaves are simultaneously active, so reaching pass
// proves the multi-target entry actually entered both legs (a single-
// leaf entry would land in s2p11/s2p12's default child and trip the
// fail transitions instead).
//
// Spec-mirror parity (cpp HierarchicalStateHelper::buildEntryChain
// pushes parallel regions onto the chain; C11 expands them inline
// because its chain[] is a fixed-depth walk-up array).

#include <stdio.h>

#include "test413_sm.h"

int main(void) {
    test413_t sm;
    test413_init(&sm);
    test413_run(&sm);

    int rc = test413_in_state(&sm, TEST413_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test413: FAIL — active = 0x%08x\n", (unsigned)test413_active_states(&sm));
    }
    test413_destroy(&sm);
    return rc;
}
