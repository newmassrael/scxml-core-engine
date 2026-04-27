// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test336 — C11 AOT runner.
//
// W3C SCXML 5.10.1: external events carry an `origin` URL plus
// `origintype` processor URI that, when handed back to <send>'s
// `targetexpr`/`typeexpr`, route an event to the originating
// session. The fixture's s0 onentry sends `foo` (bare external
// → external queue → origin = `<name>_session`, origintype =
// SCXMLEventProcessor URI). The s0 transition body sends `bar`
// with `targetexpr=_event.origin` (resolves to the same session
// URI) and `typeexpr=_event.origintype` (resolves to the
// processor URI). The new typeexpr+targetexpr combo arm
// validates the type as supported, then the targetexpr arm's
// self-session-URI clause routes `bar` back into the external
// queue. s1 onentry queues `baz` external; the App.D.2 external
// drain pops `bar` first (FIFO doc-order) and matches s1's
// `<transition event=bar target=pass>`. Without the origin/
// origintype lua globals the receiver assigns nil and the
// `<send targetexpr=nil/>` falls through the targetexpr arm's
// unreachable clause to `error.communication` → wildcard fail.

#include <stdio.h>

#include "test336_sm.h"

int main(void) {
    test336_t sm;
    test336_init(&sm);
    test336_run(&sm);

    int rc = test336_in_state(&sm, TEST336_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test336: FAIL — active = 0x%08x\n", (unsigned)test336_active_states(&sm));
    }
    test336_destroy(&sm);
    return rc;
}
