// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test570 — C11 AOT runner.
//
// Per-fixture surface description lives in backends/c/tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(570) invocation. The
// runner here is the standard C11 boilerplate; raise e1/e2 + the
// done.state internal chain drain to quiescence before the 2 s
// safety-net `<send delay="2s" event="timeout"/>` is pumped, so simple
// `_run` reaches pass without `_tick` polling.

#include <stdio.h>

#include "test570_sm.h"

int main(void) {
    test570_t sm;
    test570_init(&sm);
    test570_run(&sm);

    int rc = test570_in_state(&sm, TEST570_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test570: FAIL — active = 0x%08x\n", (unsigned)test570_active_states(&sm));
    }
    test570_destroy(&sm);
    return rc;
}
