// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test495 — C11 AOT runner.
//
// W3C SCXML 6.2 + C.1: `type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"`
// is the default and is orthogonal to queue routing — `target=""`
// (omitted) lands on the external queue, while `target="#_internal"`
// lands on the internal (high-priority) queue. The fixture sends
// event1 with the default-type literal (bare external) and event2
// with the same default-type literal plus `target="#_internal"`
// (internal). App.D.2 drains internal first, so event2 fires the
// s0→s1 transition before event1 dequeues; s1 then matches event1 to
// pass. Either inverted ordering routes through the explicit
// event1→fail transition.

#include <stdio.h>

#include "test495_sm.h"

int main(void) {
    test495_t sm;
    test495_init(&sm);
    test495_run(&sm);

    int rc = test495_in_state(&sm, TEST495_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test495: FAIL — active = 0x%08x\n", (unsigned)test495_active_states(&sm));
    }
    test495_destroy(&sm);
    return rc;
}
