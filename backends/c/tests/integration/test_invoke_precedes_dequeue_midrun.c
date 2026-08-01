// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: the invoke-before-dequeue order holds mid-run — C11 AOT path.
//
// `mainEventLoop` is one loop, so the ordering it fixes is not a property of
// start-up. Every iteration completes a macrostep, starts the invokes for the
// states that macrostep entered, and only then dequeues. `statesToInvoke` is
// filled by `enterStates`, which runs in `microstep` -- so a state entered by an
// external event's transition arms an invoke that must start before the next
// event comes off the queue.
//
// An engine that drains the external queue to exhaustion inside one step
// satisfies the start-up ordering and still loses this one: it takes the
// transition into the invoking state and then keeps draining, so what that
// state's `<onentry>` queued is consumed while the invoke is still pending.
//
// The sibling `invoke_precedes_dequeue_midrun` pins the same order at
// initialization, where the invoking state is the initial configuration and no
// transition is involved. This one reaches it through `arm` -> `phase`.
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
// Fixture: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_precedes_dequeue_midrun ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "invoke_precedes_dequeue_midrun_sm.h"

int main(void) {
    invoke_precedes_dequeue_midrun_t sm;
    invoke_precedes_dequeue_midrun_init(&sm);

    // The child opens the verdict exchange itself (`ready` from its own
    // onentry), so the parent never has to guess when the invoke completed
    // and no wall-clock delay is involved.
    invoke_precedes_dequeue_midrun_run(&sm);

    int rc = invoke_precedes_dequeue_midrun_in_state(&sm, INVOKE_PRECEDES_DEQUEUE_MIDRUN_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr,
                "invoke_precedes_dequeue_midrun: FAIL — the watching child "
                "answered `probe` from `waiting`, so it never saw `kick`. The "
                "parent drained its external queue before starting the invoke, "
                "and the event onentry had queued for itself was consumed "
                "while no child existed. W3C Appendix D `mainEventLoop` runs "
                "`invoke(inv)` for every state entered on the last iteration "
                "before it reaches `externalQueue.dequeue()`, so an "
                "autoforward child is live for the whole external queue. "
                "Diagnostic: in_PASS=%d in_FAIL=%d in_phase=%d\n",
                invoke_precedes_dequeue_midrun_in_state(&sm, INVOKE_PRECEDES_DEQUEUE_MIDRUN_STATE_PASS),
                invoke_precedes_dequeue_midrun_in_state(&sm, INVOKE_PRECEDES_DEQUEUE_MIDRUN_STATE_FAIL),
                invoke_precedes_dequeue_midrun_in_state(&sm, INVOKE_PRECEDES_DEQUEUE_MIDRUN_STATE_PHASE));
    }
    invoke_precedes_dequeue_midrun_destroy(&sm);
    return rc;
}
