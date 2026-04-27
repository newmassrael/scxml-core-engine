// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test354 — C11 AOT runner.
//
// W3C SCXML 5.10 + 6.2 + C.1: a single <send> can carry both a
// `namelist` (variables to copy by name) and one or more `<param>`
// children (named expressions). Both populate the same _event.data
// table on the receiver. test354's s0 onentry sends event1 with
// type="...#SCXMLEventProcessor" namelist="Var1" and a <param
// name="param1" expr="2"/>; the receiving transition assigns
// Var2=_event.data.Var1 (1) and Var3=_event.data.param1 (2). s1's
// cond Var2==1 advances to s2, s2's cond Var3==2 advances to s3, and
// s3's <send event="event2"><content/></send> drives the run to pass.
// The two 5 s `<send delay="5s" event="timeout"/>` siblings never
// fire because the immediate sends always reach pass first; they
// remain on the scheduler array and the registry references are
// reclaimed by destroy.

#include <stdio.h>

#include "test354_sm.h"

int main(void) {
    test354_t sm;
    test354_init(&sm);
    test354_run(&sm);

    int rc = test354_in_state(&sm, TEST354_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test354: FAIL — active = 0x%08x\n", (unsigned)test354_active_states(&sm));
    }
    test354_destroy(&sm);
    return rc;
}
