// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue — C11 AOT path.
//
// `mainEventLoop` completes the macrostep on eventless and internal
// transitions alone, then runs `invoke(inv)` for every state entered on the
// last iteration, and only then reaches `externalQueue.dequeue()`. The
// external queue is named exactly once in that loop and it is after the
// invokes.
//
// An engine that folds the external drain into its macrostep completion loop
// consumes whatever `<onentry>` queued for the parent itself while the
// invoked children do not yet exist, so an autoforward child misses every
// event the parent queued on the way in. That is a lost event, not a
// reordered one.
//
// The sibling `test_autoforward_dequeue_point.c` pins where in the loop the
// forward sits and is deliberately blind to this axis: there the child opens
// the exchange, so it is alive before anything is queued. Here the parent
// queues first and the child starts second.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_precedes_external_dequeue ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "invoke_precedes_external_dequeue_sm.h"

int main(void) {
    invoke_precedes_external_dequeue_t sm;
    invoke_precedes_external_dequeue_init(&sm);

    // The child opens the verdict exchange itself (`ready` from its own
    // onentry), so the parent never has to guess when the invoke completed
    // and no wall-clock delay is involved.
    invoke_precedes_external_dequeue_run(&sm);

    int rc = invoke_precedes_external_dequeue_in_state(&sm, INVOKE_PRECEDES_EXTERNAL_DEQUEUE_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "invoke_precedes_external_dequeue: FAIL — the watching child "
                "answered `probe` from `waiting`, so it never saw `kick`. The "
                "parent drained its external queue before starting the invoke, "
                "and the event onentry had queued for itself was consumed "
                "while no child existed. W3C Appendix D `mainEventLoop` runs "
                "`invoke(inv)` for every state entered on the last iteration "
                "before it reaches `externalQueue.dequeue()`, so an "
                "autoforward child is live for the whole external queue. "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                invoke_precedes_external_dequeue_in_state(&sm, INVOKE_PRECEDES_EXTERNAL_DEQUEUE_STATE_PASS),
                invoke_precedes_external_dequeue_in_state(&sm, INVOKE_PRECEDES_EXTERNAL_DEQUEUE_STATE_FAIL),
                invoke_precedes_external_dequeue_in_state(&sm, INVOKE_PRECEDES_EXTERNAL_DEQUEUE_STATE_PHASE));
    }
    invoke_precedes_external_dequeue_destroy(&sm);
    return rc;
}
