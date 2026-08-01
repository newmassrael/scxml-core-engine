// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — Kotlin AOT path.
//
// Appendix D's `mainEventLoop` forwards one statement after
// `externalQueue.dequeue()` and before `selectTransitions`, and §6.4.2 says
// the same in prose: the parent forwards "at the point at which it removes it
// from the external event queue". Forwarding where the event is queued
// instead breaks run-to-completion — the child sees event N before the parent
// has processed 1..N-1.
//
// Siblings `AutoforwardDoneInvokeTest` and `AutoforwardInternalQueueTest` pin
// which events are forwarded and are deliberately blind to when; this one
// pins the position and nothing else.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_dequeue_point_kotlin.sh

package com.sce.integration

import com.sce.integration.autoforward_dequeue_point.AutoforwardDequeuePointState
import com.sce.integration.autoforward_dequeue_point.AutoforwardDequeuePointStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4 — the parent forwards where it dequeues, not where it queues (Kotlin AOT).
@DisplayName("AutoforwardDequeuePoint — W3C SCXML 6.4")
class AutoforwardDequeuePointTest {

    @Test
    fun anExternalEventIsForwardedAtTheDequeueNotTheEnqueue() {
        val sm = AutoforwardDequeuePointStateMachine()
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AutoforwardDequeuePointState.Pass,
            sm.currentState.value,
            "the probe child saw `second` before `mark`, so both events were handed over " +
                "while the parent was still executing the transition that queued them. " +
                "W3C Appendix D `mainEventLoop` forwards one statement after " +
                "`externalQueue.dequeue()`, and §6.4.2 puts it \"at the point at which it " +
                "removes it from the external event queue\" — forwarding at the enqueue " +
                "lets the child run ahead of the parent by a whole event."
        )
    }
}
