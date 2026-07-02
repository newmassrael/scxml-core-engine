// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test403c — C11 AOT runner.
//
// Optimal enabled set with cross-compound preemption inside a <parallel>.
// On event1: p0s2's external transition `<transition target="p0s1">`
// (raises event2) preempts p0s3→fail because the cpp-mirrored exit set
// for p0s3→fail walks up to LCA(p0s3, fail) = scxml — its bitmap
// includes the parallel ancestor p0, so removeConflictingTransitions
// criterion (c) fires, doc-order picks p0s2 as the survivor. p0s4's
// wildcard transition `<transition event="*">` runs unpreempted because
// targetless transitions emit is_internal=true and source==target=p0s4,
// triggering the (c) self-loop exemption (Var1 increments to 1).
// On event2 (raised by p0s2's transition body): p0s3→s1 preempts the
// inherited s0→fail via the descendant-source preemption rule, p0s4
// wildcard fires again (Var1 == 2), entry into s1 satisfies the pass
// cond. The fixture pins the find_lca + LCA-based compute_exit_set
// generalization plus the parallel-sibling re-entry pass that follows
// the multi-region exit (W3C SCXML 3.4 / App.D.2 / 3.13).
//
// `<send delay="1s">` is a safety-net timeout — silent stub via the
// emit_action send dimension matrix carve-out (the success path never
// fires it, so the deferred scheduler infrastructure is unforced here).

#include <stdio.h>

#include "test403c_sm.h"

int main(void) {
    test403c_t sm;
    test403c_init(&sm);
    test403c_run(&sm);

    int rc = test403c_in_state(&sm, TEST403C_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test403c: FAIL — active = 0x%08x\n", (unsigned)test403c_active_states(&sm));
    }
    test403c_destroy(&sm);
    return rc;
}
