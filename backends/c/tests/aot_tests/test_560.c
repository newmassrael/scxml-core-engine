// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test560 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2: `<send event="foo"><param name="aParam" expr="1"/></send>`
// with a single literal-1 param, no datamodel block at all. The receiving
// transition's cond `_event.data.aParam == 1` reads the param off the
// promoted lua table on `_event.data` — fixture exists to verify the C11
// send-param arm works without a top-level `<datamodel>`, only the
// `_event.data.aParam` reader path forces `needs_script_engine`.

#include <stdio.h>

#include "test560_sm.h"

int main(void) {
    test560_t sm;
    test560_init(&sm);
    test560_run(&sm);

    int rc = test560_in_state(&sm, TEST560_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test560: FAIL — active = 0x%08x\n", (unsigned)test560_active_states(&sm));
    }
    test560_destroy(&sm);
    return rc;
}
