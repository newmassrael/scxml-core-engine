// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out — Kotlin AOT.
//
// Appendix D's main event loop exits when the machine reaches a top-level final
// state. Refusing what arrives afterwards is the clause; saying nothing about
// it is not. The silence is expensive because it looks like the two outcomes a
// host can already read:
//
//   dequeued, no transition matched            discardedExternalEvents()
//   dequeued, matched, guard said no           nothing, correctly
//   never dequeued — the machine had stopped   this
//
// This backend had a second way to lose the event. In coroutine mode `send`
// writes to a channel that is CLOSED once the machine finishes, so the delivery
// did not even reach a queue — the failure was a `trySend` result nobody read.
//
// Fixture: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unseen_event_is_reported_kotlin.sh

package com.sce.integration

import com.sce.integration.unseen_event_is_reported.UnseenEventIsReportedEvent
import com.sce.integration.unseen_event_is_reported.UnseenEventIsReportedStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.13 — an event a stopped machine never looked at is something the host can see.
@DisplayName("UnseenEventIsReported — W3C SCXML 3.13")
class UnseenEventIsReportedTest {

    private fun started(): UnseenEventIsReportedStateMachine {
        // The fixture counts handled deliveries with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = UnseenEventIsReportedStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: UnseenEventIsReportedStateMachine, event: UnseenEventIsReportedEvent) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: an event the host queued after the machine stopped is counted.
    @Test
    fun anEventDeliveredAfterTheMachineStoppedIsCounted() {
        val sm = started()
        assertEquals(0, sm.unseenExternalEvents(), "nothing has been refused before the first event")

        deliver(sm, UnseenEventIsReportedEvent.Poke)
        assertEquals(
            1L,
            sm.pokes(),
            "`poke`'s transition did not run, so nothing below is measuring a machine that " +
                "was working first"
        )

        deliver(sm, UnseenEventIsReportedEvent.Finish)
        assertTrue(sm.isInFinalState, "`finish` should have taken the machine to its top-level final state")
        assertEquals(
            0,
            sm.unseenExternalEvents(),
            "`finish` was itself dequeued and handled — the machine stopped BECAUSE of it, " +
                "which is not the same as stopping before it"
        )

        deliver(sm, UnseenEventIsReportedEvent.Poke)

        assertEquals(
            1,
            sm.unseenExternalEvents(),
            "the host queued `poke` on a machine that had reached its final state. W3C SCXML " +
                "Appendix D's loop had already ended, so the event was never dequeued; before " +
                "this count the host had no way to learn that"
        )
        assertEquals(
            1L,
            sm.pokes(),
            "the refused delivery ran the document's transition anyway — the count would then " +
                "be reporting something that did not happen"
        )
    }

    /// Why the query has to exist at all: every other accessor answers the same
    /// before and after the refused delivery.
    @Test
    fun theRefusalIsNotDerivableFromAnyOtherAccessor() {
        val sm = started()
        deliver(sm, UnseenEventIsReportedEvent.Finish)

        val beforeState = sm.currentState.value
        val beforeFinal = sm.isInFinalState
        val beforeDiscarded = sm.discardedExternalEvents()
        val beforePokes = sm.pokes()

        deliver(sm, UnseenEventIsReportedEvent.Poke)

        assertEquals(
            beforeState,
            sm.currentState.value,
            "this fixture exists because a refused delivery is indistinguishable through the " +
                "accessors a host had; if they ever differ, the fixture stopped measuring " +
                "what it claims"
        )
        assertEquals(beforeFinal, sm.isInFinalState)
        assertEquals(beforeDiscarded, sm.discardedExternalEvents())
        assertEquals(beforePokes, sm.pokes())

        assertEquals(
            1,
            sm.unseenExternalEvents(),
            "the two readings agree on everything else, so this count is the only thing that " +
                "separates `the machine never looked` from `it looked and nothing matched`"
        )
    }

    /// The distinction the whole axis turns on: a discard and a refusal are
    /// different facts, and each has its own count.
    @Test
    fun aDiscardAndARefusalAreCountedSeparately() {
        val sm = started()

        deliver(sm, UnseenEventIsReportedEvent.Poke)
        assertEquals(0, sm.discardedExternalEvents(), "`poke` matched a targetless transition")
        assertEquals(0, sm.unseenExternalEvents(), "the machine was running, so nothing was refused")

        deliver(sm, UnseenEventIsReportedEvent.Finish)
        deliver(sm, UnseenEventIsReportedEvent.Poke)

        assertEquals(
            0,
            sm.discardedExternalEvents(),
            "a refusal must not be reported as a discard: the first says the machine looked " +
                "and nothing matched, the second says it never looked"
        )
        assertEquals(1, sm.unseenExternalEvents())
    }

    /// A count says an event went unlooked-at; a host debugging a supervisor
    /// that stopped answering needs to know which one.
    @Test
    fun theEngineNamesTheEventItNeverLookedAt() {
        val sm = started()
        assertNull(sm.lastUnseenEvent(), "nothing has been refused yet")

        deliver(sm, UnseenEventIsReportedEvent.Finish)
        deliver(sm, UnseenEventIsReportedEvent.Poke)
        assertEquals(
            UnseenEventIsReportedEvent.Poke,
            sm.lastUnseenEvent(),
            "the engine counted a refusal but cannot say which event it refused"
        )

        deliver(sm, UnseenEventIsReportedEvent.Finish)
        assertEquals(2, sm.unseenExternalEvents(), "the count is a count, not a flag")
        assertEquals(
            UnseenEventIsReportedEvent.Finish,
            sm.lastUnseenEvent(),
            "the name did not follow the second refusal"
        )
    }
}
