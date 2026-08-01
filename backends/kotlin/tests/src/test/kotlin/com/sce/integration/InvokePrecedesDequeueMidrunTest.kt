// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: the invoke-before-dequeue order holds mid-run — Kotlin AOT path.
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
// consumes whatever `<onentry>` queued for the parent itself while the invoked
// children do not yet exist, so an autoforward child misses every event the
// parent queued on the way in. That is a lost event, not a reordered one.
//
// The sibling `AutoforwardDequeuePointTest` pins where in the loop the forward
// sits and is deliberately blind to this axis: there the child opens the
// exchange, so it is alive before anything is queued. Here the parent queues
// first and the child starts second.
//
// Fixture: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_precedes_dequeue_midrun_kotlin.sh

package com.sce.integration

import com.sce.integration.invoke_precedes_dequeue_midrun.InvokePrecedesDequeueMidrunState
import com.sce.integration.invoke_precedes_dequeue_midrun.InvokePrecedesDequeueMidrunStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML Appendix D — the parent starts its invokes before it dequeues (Kotlin AOT).
@DisplayName("InvokePrecedesDequeueMidrun — W3C SCXML Appendix D")
class InvokePrecedesDequeueMidrunTest {

    @Test
    fun pendingInvokesStartBeforeTheDequeueMidRun() {
        val sm = InvokePrecedesDequeueMidrunStateMachine()
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            InvokePrecedesDequeueMidrunState.Pass,
            sm.currentState.value,
            "the watching child answered `probe` from `waiting`, so it never saw `kick`. " +
                "The parent drained its external queue before starting the invoke, and the " +
                "event `<onentry>` had queued for itself was consumed while no child " +
                "existed. W3C Appendix D `mainEventLoop` runs `invoke(inv)` for every state " +
                "entered on the last iteration before it reaches `externalQueue.dequeue()`, " +
                "so an autoforward child is live for the whole external queue."
        )
    }
}
