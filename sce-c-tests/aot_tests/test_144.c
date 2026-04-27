// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test144 — C11 AOT runner.
//
// W3C SCXML 4.2: <raise> inserts events at the rear of the internal
// queue. The fixture's onentry raises `foo` then `bar`; the engine must
// process them FIFO — so s0 fires its `foo` transition into s1, and s1
// fires its `bar` transition into pass. A LIFO bug would route bar back
// through s0's wildcard fallback to fail.

#include <stdio.h>

#include "test144_sm.h"

int main(void) {
    test144_t sm;
    test144_init(&sm);
    test144_run(&sm);

    if (test144_in_state(&sm, TEST144_STATE_PASS)) {
        return 0;
    }
    fprintf(stderr, "test144: FAIL — active = 0x%08x\n", (unsigned)test144_active_states(&sm));
    return 1;
}
