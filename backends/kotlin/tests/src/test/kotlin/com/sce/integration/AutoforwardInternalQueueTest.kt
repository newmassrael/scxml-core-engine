// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — Kotlin AOT path.
//
// Appendix D's `mainEventLoop` forwards only what it dequeues from the
// external queue; the internal drain above it has no forwarding step at
// all. §6.2 raises `error.execution` onto the internal queue when `<send>`
// names an unsupported type, so it must never reach an `autoforward`
// child — and it must be excluded by where it was raised, not by a filter
// that recognises its name.
//
// Sibling of `AutoforwardDoneInvokeTest`, which pins the positive half.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_internal_queue_kotlin.sh

package com.sce.integration

import com.sce.integration.autoforward_internal_queue.AutoforwardInternalQueueState
import com.sce.integration.autoforward_internal_queue.AutoforwardInternalQueueStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4 — an internal-queue event never reaches an autoforward child (Kotlin AOT).
@DisplayName("AutoforwardInternalQueue — W3C SCXML 6.4")
class AutoforwardInternalQueueTest {

    @Test
    fun anInternalQueueEventIsNeverAutoforwarded() {
        val sm = AutoforwardInternalQueueStateMachine()
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AutoforwardInternalQueueState.Pass,
            sm.currentState.value,
            "the watcher saw `error.execution`: an internal-queue event was autoforwarded. " +
                "W3C Appendix D `mainEventLoop` forwards only what it dequeues from the " +
                "external queue, and §6.2 raises `error.execution` onto the internal one — " +
                "check that the event was not routed onto the external queue for some " +
                "unrelated reason, which would leak it past any name-blind forward."
        )
    }
}
