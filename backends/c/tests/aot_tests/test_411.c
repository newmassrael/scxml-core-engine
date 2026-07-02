// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test411 — C11 AOT runner.
//
// W3C SCXML 5.9.2 + 3.13: In() inside `<onexit>`-driven `<if>`. When
// s01's onexit runs the cond `In('s01')` must already read **false**
// because by that point the runtime has begun exiting s01 — its bit is
// cleared from the configuration before exit-actions execute (W3C 3.4
// "exit set"). The native lower (filter `to_in_predicate_c11`) reads the
// live `active` bitmap at the moment the `<if>` fires; the safety-net
// `<send delay="1s" event="timeout"/>` only fires if the eventless path
// stalls, so `_run` quiesces well under the cap.

#include <stdio.h>

#include "test411_sm.h"

int main(void) {
    test411_t sm;
    test411_init(&sm);
    test411_run(&sm);

    int rc = test411_in_state(&sm, TEST411_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test411: FAIL — active = 0x%08x\n", (unsigned)test411_active_states(&sm));
    }
    test411_destroy(&sm);
    return rc;
}
