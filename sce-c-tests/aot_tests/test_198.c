// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test198 — C11 AOT runner.
//
// W3C SCXML 6.2 + C.1: bare `<send>` without a `type` attribute defaults to the SCXML Event I/O Processor — the C11 emit's bare-external dispatch path (b66024fe, no-target/no-type elif arm) drops the event into the external queue, the s0 transition matches, and the receiving cond `_event.origintype` reads back the SCXMLEventProcessor URI carved into _pending_event_origintype on each external pop.

#include <stdio.h>

#include "test198_sm.h"

int main(void) {
    test198_t sm;
    test198_init(&sm);
    test198_run(&sm);

    int rc = test198_in_state(&sm, TEST198_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test198: FAIL — active = 0x%08x\n", (unsigned)test198_active_states(&sm));
    }
    test198_destroy(&sm);
    return rc;
}
