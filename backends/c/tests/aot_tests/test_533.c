// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test533 — C11 AOT runner.
//
// W3C SCXML 3.13: an internal transition whose source is a non-compound (atomic) state still exits its source — there
// is no special-case carve-out for atomic source states. The compute_exit_set internal-to-descendant branch (no-op for
// atomic source) covers the case; the receiving transition confirms the source's onexit fired exactly once.

#include <stdio.h>

#include "test533_sm.h"

int main(void) {
    test533_t sm;
    test533_init(&sm);
    test533_run(&sm);

    int rc = test533_in_state(&sm, TEST533_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test533: FAIL — active = 0x%08x\n", (unsigned)test533_active_states(&sm));
    }
    test533_destroy(&sm);
    return rc;
}
