// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test417 — C11 AOT runner.
//
// Per-fixture surface description lives in sce-c-tests/CMakeLists.txt
// alongside the sce_generate_static_w3c_c_test(417) invocation. The
// runner here is the standard C11 boilerplate; the eventless transition
// chain (s1p111→s1p11final + s1p121→s1p12final) drains to quiescence
// before the 1 s safety-net `<send delay="1s" event="timeout"/>` is
// pumped, so simple `_run` reaches pass without `_tick` polling.

#include <stdio.h>

#include "test417_sm.h"

int main(void) {
    test417_t sm;
    test417_init(&sm);
    test417_run(&sm);

    int rc = test417_in_state(&sm, TEST417_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test417: FAIL — active = 0x%08x\n", (unsigned)test417_active_states(&sm));
    }
    test417_destroy(&sm);
    return rc;
}
