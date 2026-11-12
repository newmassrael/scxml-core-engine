// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test388 — C11 AOT runner.
//
// W3C SCXML 3.10: history snapshot capture/restore — re-entering s0 via deep
// history restores s012 (last active leaf), shallow history restores s01 then
// walks initial s011
//
// The runner here is the standard C11 boilerplate: init, run, compare the
// final state against TEST388_STATE_PASS, return 0 on PASS / 1 on FAIL. What
// it links beside the machine is what the generator states the machine
// requires (backends/c/tests/CMakeLists.txt).

#include <stdio.h>

#include "../require_started.h"
#include "test388_sm.h"

int main(void) {
    test388_t sm;
    REQUIRE_STARTED(test388_init(&sm), TEST388_START_OK);
    test388_run(&sm);

    int rc = test388_ended_in(&sm, TEST388_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test388: FAIL — active = 0x%08x\n", (unsigned)test388_active_states(&sm));
    }
    test388_destroy(&sm);
    return rc;
}
