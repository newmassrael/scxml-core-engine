// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — C11 AOT path.
//
// Appendix D's `mainEventLoop` forwards one statement after
// `externalQueue.dequeue()` and before `selectTransitions`, and 6.4.2 says
// the same in prose: the parent forwards "at the point at which it removes
// it from the external event queue". Forwarding where the event is queued
// instead breaks run-to-completion — the child sees event N before the
// parent has processed 1..N-1.
//
// Siblings `test_autoforward_done_invoke.c` and
// `test_autoforward_internal_queue.c` pin which events are forwarded and
// are deliberately blind to when; this one pins the position and nothing
// else.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(autoforward_dequeue_point ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "autoforward_dequeue_point_sm.h"

int main(void) {
    autoforward_dequeue_point_t sm;
    autoforward_dequeue_point_init(&sm);

    // The child opens the exchange (`ready` from its own onentry), so it is
    // provably alive for everything that follows and the verdict does not
    // depend on when this backend starts its pending invokes relative to
    // the external drain.
    autoforward_dequeue_point_run(&sm);

    int rc = autoforward_dequeue_point_in_state(&sm, AUTOFORWARD_DEQUEUE_POINT_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "autoforward_dequeue_point: FAIL — the probe child saw "
                "`second` before `mark`, so both events were handed over "
                "while the parent was still executing the transition that "
                "queued them. W3C Appendix D `mainEventLoop` forwards one "
                "statement after `externalQueue.dequeue()`, and 6.4.2 puts "
                "it at the point at which the parent removes the event from "
                "the external event queue: forwarding at the enqueue lets "
                "the child run ahead of the parent by a whole event. "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                autoforward_dequeue_point_in_state(&sm, AUTOFORWARD_DEQUEUE_POINT_STATE_PASS),
                autoforward_dequeue_point_in_state(&sm, AUTOFORWARD_DEQUEUE_POINT_STATE_FAIL),
                autoforward_dequeue_point_in_state(&sm, AUTOFORWARD_DEQUEUE_POINT_STATE_PHASE));
    }
    autoforward_dequeue_point_destroy(&sm);
    return rc;
}
