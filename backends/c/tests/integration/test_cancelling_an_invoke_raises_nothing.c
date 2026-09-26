// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — C11 AOT.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Fixture: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(cancelling_an_invoke_raises_nothing ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "cancelling_an_invoke_raises_nothing_sm.h"

typedef cancelling_an_invoke_raises_nothing_t sm_t;

static void deliver(sm_t *sm, cancelling_an_invoke_raises_nothing_event_t event) {
    cancelling_an_invoke_raises_nothing_event_with_meta_t carrier = {0};
    carrier.event = event;
    cancelling_an_invoke_raises_nothing_raise_external(sm, &carrier);
    cancelling_an_invoke_raises_nothing_run(sm);
}

int main(void) {
    sm_t sm;
    cancelling_an_invoke_raises_nothing_init(&sm);
    cancelling_an_invoke_raises_nothing_run(&sm);
    if (!cancelling_an_invoke_raises_nothing_in_state(&sm, CANCELLING_AN_INVOKE_RAISES_NOTHING_STATE_P)) {
        fprintf(stderr, "FAIL: the run has to start in `p`, with its child invoked\n");
        return 1;
    }

    deliver(&sm, CANCELLING_AN_INVOKE_RAISES_NOTHING_EVENT_LEAVE);
    deliver(&sm, CANCELLING_AN_INVOKE_RAISES_NOTHING_EVENT_FINISH);

    int ok = 1;
    if (!cancelling_an_invoke_raises_nothing_ended_in(&sm, CANCELLING_AN_INVOKE_RAISES_NOTHING_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }
    int64_t spurious = -1;
    if (!cancelling_an_invoke_raises_nothing_spurious(&sm, &spurious) || spurious != 0) {
        fprintf(stderr,
                "FAIL: leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session "
                "(spurious=%lld); W3C SCXML 6.4 defines no such event\n",
                (long long)spurious);
        ok = 0;
    }
    cancelling_an_invoke_raises_nothing_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: cancelling an invocation raised no event in the invoking session\n");
    return 0;
}
