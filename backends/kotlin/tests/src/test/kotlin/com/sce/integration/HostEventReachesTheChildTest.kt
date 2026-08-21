// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — Kotlin AOT path.
//
// The four sibling `autoforward_*` stems all let the machine forward events it
// queued for itself. This one hands it one from outside, through the engine's
// own "here is an event" entry point, and asks whether the `autoforward` child
// sees it. Appendix D's `mainEventLoop` binds the preliminary step
// (`applyFinalize` plus the autoforward `send`) to the external event it is
// about to select transitions for, so an engine with a second door has to run
// the step at both or the child goes blind to everything the host delivers.
//
// Measured 2026-08-21: the C++ AOT engine had the step written inline in its
// queue drain, so `processEvent()` skipped it. This engine's `send` appends to
// the external queue and `tick()` drains it, so the drain is its only door and
// this pins that — a later entry point that hands the event straight to the
// transition selector would go red here.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_event_reaches_the_child_kotlin.sh

package com.sce.integration

import com.sce.integration.host_event_reaches_the_child.HostEventReachesTheChildEvent
import com.sce.integration.host_event_reaches_the_child.HostEventReachesTheChildState
import com.sce.integration.host_event_reaches_the_child.HostEventReachesTheChildStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4 — an event the host hands over reaches the autoforward child (Kotlin AOT).
@DisplayName("HostEventReachesTheChild — W3C SCXML 6.4")
class HostEventReachesTheChildTest {

    @Test
    fun anEventTheHostHandsOverReachesTheAutoforwardChild() {
        val sm = HostEventReachesTheChildStateMachine()
        sm.initialize()

        // The child opens the exchange, so let its `ready` move the parent into
        // `armed` — the one state that can be handed an event from outside.
        val armedDeadline = System.currentTimeMillis() + 2000L
        while (sm.currentState.value !is HostEventReachesTheChildState.Armed &&
            System.currentTimeMillis() < armedDeadline
        ) {
            Thread.sleep(10)
            sm.tick()
        }
        assertEquals(
            HostEventReachesTheChildState.Armed,
            sm.currentState.value,
            "the probe child never sent `ready`, so the fixture never reached the state " +
                "where a host event can be handed over — this is a broken handshake, not " +
                "a forwarding verdict"
        )

        // The axis: the host's own entry point.
        sm.send(HostEventReachesTheChildEvent.HostPing)

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            Thread.sleep(10)
            sm.tick()
        }

        assertEquals(
            HostEventReachesTheChildState.Pass,
            sm.currentState.value,
            "the probe child answered `sawMarkerOnly`, so the event the host handed to " +
                "`send` was never forwarded to it and the child only ever saw the `marker` " +
                "the parent's own transition body sent. W3C Appendix D `mainEventLoop` runs " +
                "the autoforward `send` against the external event before it selects " +
                "transitions for it, whichever door the event arrived through — an engine " +
                "that runs that step only in its queue drain leaves an `autoforward` child " +
                "blind to everything its host delivers."
        )
    }
}
