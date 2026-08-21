// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Kotlin AOT
// path.
//
// This engine did not spin on the `<raise>` half either — `drainEventlessAndInternal`
// stopped after a hundred internal iterations — and that is the sharper half
// of the finding: bounded and silent reads to the host exactly like unbounded.
// It also kept the two branches on separate counters, which is the case the
// `alternate` outcome below exists to catch: a document that takes every other
// microstep on each branch never reaches either ceiling.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_internal_chain_is_bounded_kotlin.sh

package com.sce.integration

import com.sce.integration.internal_chain_is_bounded.InternalChainIsBoundedEvent
import com.sce.integration.internal_chain_is_bounded.InternalChainIsBoundedState
import com.sce.integration.internal_chain_is_bounded.InternalChainIsBoundedStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.13 — a macrostep whose `<raise>` chain cannot end is stopped,
/// and the host is told.
@DisplayName("InternalChainIsBounded — W3C SCXML 3.13")
class InternalChainIsBoundedTest {

    /** The ceiling the engine applies, spelled here rather than read back from
     * it. A test that asked the engine for its own limit would agree with any
     * limit, including one an edit moved by three orders of magnitude. */
    private val maxMicrosteps = 1000L

    /** One lap of the alternating chain is two microsteps — one internal
     * event, one eventless transition — and only the internal half is counted,
     * so a chain run to the shared ceiling records half. */
    private val alternatingLapsAtCeiling = maxMicrosteps / 2

    private fun started(): InternalChainIsBoundedStateMachine {
        // The fixture counts chain links with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = InternalChainIsBoundedStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: InternalChainIsBoundedStateMachine, event: InternalChainIsBoundedEvent) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: the macrostep is cut at a known microstep, and the count says
    /// it was.
    @Test
    fun aRaiseChainThatCannotEndIsStopped() {
        val sm = started()
        assertEquals(0, sm.truncatedMacrosteps(), "nothing has been refused before the machine has done anything")

        deliver(sm, InternalChainIsBoundedEvent.Spin)

        assertEquals(
            maxMicrosteps,
            sm.links(),
            "the chain must run exactly as far as the engine allows: fewer means the document " +
                "was cut off early, more means the ceiling moved"
        )
        assertEquals(
            1,
            sm.truncatedMacrosteps(),
            "the microstep past the budget was queued and was not taken. Without this count " +
                "the host sees a machine in a state the document names, having returned at once, " +
                "with no way to learn that the configuration it is reading is not a stable one"
        )
        assertEquals(
            InternalChainIsBoundedState.Spin,
            sm.lastTruncatedMacrostepState(),
            "the count alone says a document somewhere cannot settle; this says where to look"
        )
        // This engine publishes no `isRunning`, so the sibling channels'
        // "the chain was cut, not the machine" is read here as the machine
        // still standing where the chain left it — which a stopped one would
        // not be able to answer.
        assertTrue(
            sm.currentState.value == InternalChainIsBoundedState.Spin,
            "the chain was cut, not the machine: the specification allows the document, so " +
                "refusing to run it forever is the engine's decision to report, not a reason to " +
                "stop a machine whose other states still work"
        )
    }

    /// The other half, and the one that makes the count mean something: a
    /// chain that ends on its own is not refused, however long it is.
    @Test
    fun aRaiseChainThatEndsAtTheCeilingIsNotRefused() {
        val sm = started()

        deliver(sm, InternalChainIsBoundedEvent.Bounded)

        assertEquals(
            maxMicrosteps,
            sm.laps(),
            "the guard `laps < 999` stops matching at the thousandth link, which raises nothing, " +
                "so the queue empties and the chain stops by itself"
        )
        assertEquals(
            0,
            sm.truncatedMacrosteps(),
            "nothing was refused: the macrostep reached the stable configuration the clause " +
                "describes, using every microstep it was allowed. A long chain is not a runaway"
        )
        assertNull(sm.lastTruncatedMacrostepState(), "and nothing names a state, because nothing was stopped")
        assertTrue(
            sm.currentState.value == InternalChainIsBoundedState.Bounded,
            "a document that settles on its own rests where its chain ended, and must not be " +
                "reported dead by an engine that just finished running it correctly"
        )
    }

    /// A dequeue that selected nothing is not a microstep, so it spends no
    /// budget. §scxml-D takes a microstep for a transition that was SELECTED;
    /// a dequeue that matched none is the loop turn the clause does not count.
    /// The fixture's `unanswered` chain is `bounded` with one unmatched event
    /// added per link, so the two differ in exactly that and must cost the
    /// same.
    ///
    /// Measured 2026-08-21: this claim had no witness in any channel. The
    /// mutation that spends the budget on every dequeue SURVIVED all five
    /// outcomes, because every other chain here answers every event it raises.
    @Test
    fun aDequeueThatSelectedNothingSpendsNoBudget() {
        val sm = started()

        deliver(sm, InternalChainIsBoundedEvent.Unanswered)

        assertEquals(
            maxMicrosteps,
            sm.ignores(),
            "the chain is the same length as `bounded`; the unmatched events between its links " +
                "are dequeues that selected nothing, and those are not microsteps"
        )
        assertEquals(
            0,
            sm.truncatedMacrosteps(),
            "a thousand microsteps and a thousand discards is a thousand microsteps: an engine " +
                "that counted the discards refuses this document at link five hundred and " +
                "reports a runaway that is not one"
        )
        assertNull(sm.lastTruncatedMacrostepState(), "and nothing names a state, because nothing was stopped")
        assertTrue(
            sm.currentState.value == InternalChainIsBoundedState.Ignoring,
            "the document settled on its own and rests where its chain ended"
        )
    }

    /// The case a per-branch budget lets through: a chain that alternates one
    /// `<raise>` with one eventless transition. This engine is the one that
    /// shipped a counter per branch, so this is the assertion it used to fail
    /// by running forever.
    @Test
    fun anAlternatingChainSpendsOneSharedBudget() {
        val sm = started()

        deliver(sm, InternalChainIsBoundedEvent.Alternate)

        assertEquals(
            alternatingLapsAtCeiling,
            sm.alts(),
            "the two branches share one budget, so a chain that alternates them gets five " +
                "hundred laps out of a thousand microsteps. A thousand here would mean the " +
                "internal branch had a ceiling of its own"
        )
        assertEquals(
            1,
            sm.truncatedMacrosteps(),
            "and the refusal is reported once, whichever branch was holding the budget when it ran out"
        )
        assertEquals(
            InternalChainIsBoundedState.Alt,
            sm.lastTruncatedMacrostepState(),
            "named the same way as any other chain that could not settle"
        )
    }

    /// What the refusal did with the links it would not run: it left them
    /// queued. The fixture's `resume` chain is half again as long as the
    /// ceiling, so the first macrostep is refused with five hundred links
    /// still to go and the second one finishes them.
    ///
    /// The event driving the second macrostep is `poke`, and what it does is
    /// deliberately not asserted: internal events outrank it here, while the
    /// C++ AOT engine's `processEvent` takes the host's event first. That
    /// divergence is its own debt — the counters below are the same on both.
    @Test
    fun aRefusedChainIsLeftQueuedForTheNextMacrostep() {
        val sm = started()

        deliver(sm, InternalChainIsBoundedEvent.Resume)
        assertEquals(
            maxMicrosteps,
            sm.beats(),
            "the first macrostep spends the whole budget on the chain"
        )
        assertEquals(1, sm.truncatedMacrosteps())

        deliver(sm, InternalChainIsBoundedEvent.Poke)

        assertEquals(
            maxMicrosteps + maxMicrosteps / 2,
            sm.beats(),
            "the second macrostep picked the chain up where the first was cut and ran it to its " +
                "end — the refused links were left on the queue, not dropped"
        )
        assertEquals(
            1,
            sm.truncatedMacrosteps(),
            "and nothing was refused this time: the chain ended on its own inside the budget, " +
                "which is an ordinary macrostep however long the document took to get there"
        )
        assertTrue(
            sm.currentState.value == InternalChainIsBoundedState.Resuming,
            "the chain was cut, not the machine"
        )
    }

    /// The control: an ordinary document is untouched by any of this.
    @Test
    fun anOrdinaryMacrostepIsNotCounted() {
        val sm = started()

        deliver(sm, InternalChainIsBoundedEvent.Poke)

        assertEquals(
            1L,
            sm.pokes(),
            "the run happened: a counter of zero cannot tell an engine that did nothing from " +
                "one that was never asked"
        )
        assertEquals(0, sm.truncatedMacrosteps(), "and one transition is not a chain that cannot end")
        assertNull(sm.lastTruncatedMacrostepState())
        assertEquals(InternalChainIsBoundedState.Idle, sm.currentState.value)
    }
}
