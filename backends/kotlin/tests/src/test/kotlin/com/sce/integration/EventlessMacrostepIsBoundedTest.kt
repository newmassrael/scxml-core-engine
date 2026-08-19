// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Kotlin AOT
// path.
//
// This engine did not spin — `drainEventlessAndInternal` stopped after a
// hundred iterations — and that is the sharper half of the finding: bounded
// and silent reads to the host exactly like unbounded. The macrostep was cut
// and nothing said so, so a supervisor watching a document that cannot settle
// saw a machine that had gone quiet in a state the document names.
//
// `error_cascade_is_bounded` owns the chain built from errors; this one owns
// the chain built from transitions that need no event at all. The fixture
// separates a chain that stops on its own — a HUNDRED microsteps, exactly the
// ceiling, which is where an off-by-one lands — from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_eventless_macrostep_is_bounded_kotlin.sh

package com.sce.integration

import com.sce.integration.eventless_macrostep_is_bounded.EventlessMacrostepIsBoundedEvent
import com.sce.integration.eventless_macrostep_is_bounded.EventlessMacrostepIsBoundedState
import com.sce.integration.eventless_macrostep_is_bounded.EventlessMacrostepIsBoundedStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.13 — a macrostep whose eventless chain cannot end is stopped,
/// and the host is told.
@DisplayName("EventlessMacrostepIsBounded — W3C SCXML 3.13")
class EventlessMacrostepIsBoundedTest {

    /** The ceiling the engine applies, spelled here rather than read back from
     * it. A test that asked the engine for its own limit would agree with any
     * limit, including one an edit moved by three orders of magnitude. */
    private val maxMicrosteps = 100L

    /** One lap of either chain is two microsteps (`_a` to `_b`, then back) and
     * only the `_a` edge counts, so a chain run to the ceiling records half. */
    private val lapsAtCeiling = maxMicrosteps / 2

    private fun started(): EventlessMacrostepIsBoundedStateMachine {
        // The fixture counts chain laps with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = EventlessMacrostepIsBoundedStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: EventlessMacrostepIsBoundedStateMachine, event: EventlessMacrostepIsBoundedEvent) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: the macrostep is cut at a known microstep, and the count says
    /// it was.
    @Test
    fun aMacrostepThatCannotEndIsStopped() {
        val sm = started()
        assertEquals(0, sm.truncatedMacrosteps(), "nothing has been refused before the machine has done anything")

        deliver(sm, EventlessMacrostepIsBoundedEvent.Spin)

        assertEquals(
            lapsAtCeiling,
            sm.spins(),
            "the chain must run exactly as far as the engine allows: fewer means the document " +
                "was cut off early, more means the ceiling moved"
        )
        assertEquals(
            1,
            sm.truncatedMacrosteps(),
            "the hundred-and-first microstep was enabled and was not taken. Without this count " +
                "the host sees a machine in a state the document names, having returned at once, " +
                "with no way to learn that the configuration it is reading is not a stable one"
        )
        assertEquals(
            EventlessMacrostepIsBoundedState.SpinA,
            sm.lastTruncatedMacrostepState(),
            "an eventless cycle is a closed walk through the state graph, and the count alone " +
                "does not say which walk. This names a state on it, which is where an author looks"
        )
    }

    /// The other half, and the one that makes the count mean something: a
    /// chain that ends on its own is not refused, however long it is.
    ///
    /// The fixture's bounded chain is exactly `maxMicrosteps` microsteps for
    /// this reason. A ceiling that counted loop turns rather than microsteps
    /// taken, or that tested `>=` where it meant `>`, reports this ordinary
    /// document as a runaway.
    @Test
    fun aChainThatEndsAtTheCeilingIsNotRefused() {
        val sm = started()

        deliver(sm, EventlessMacrostepIsBoundedEvent.Bounded)

        assertEquals(
            lapsAtCeiling,
            sm.laps(),
            "the guard `laps < 50` closes after fifty laps, so the chain is a hundred microsteps " +
                "long and then stops by itself"
        )
        assertEquals(
            0,
            sm.truncatedMacrosteps(),
            "nothing was refused: the macrostep reached the stable configuration §scxml-3.13 " +
                "describes, using every microstep it was allowed. A long chain is not a runaway"
        )
        assertNull(sm.lastTruncatedMacrostepState(), "and nothing names a state, because nothing was stopped")
        assertTrue(
            sm.currentState.value == EventlessMacrostepIsBoundedState.BoundedA,
            "the chain rests where its guard closed"
        )
    }

    /// A count, not a flag: a second unbounded macrostep is refused the same
    /// way the first was.
    @Test
    fun aSecondTruncatedMacrostepCountsAgain() {
        val sm = started()

        deliver(sm, EventlessMacrostepIsBoundedEvent.Spin)
        assertEquals(1, sm.truncatedMacrosteps(), "precondition: this test is about the SECOND refusal")

        // `reset` is the fixture's way back out of the cycle, and it moves the
        // machine on purpose: the two C++ engines complete a macrostep only
        // after a transition that does.
        deliver(sm, EventlessMacrostepIsBoundedEvent.Reset)
        assertTrue(
            sm.currentState.value == EventlessMacrostepIsBoundedState.Idle,
            "reset is the way back out of the chain"
        )

        deliver(sm, EventlessMacrostepIsBoundedEvent.Spin)

        assertEquals(
            2,
            sm.truncatedMacrosteps(),
            "the second macrostep hit the same ceiling and must be counted again — a count that " +
                "saturated at one would read as a machine that recovered"
        )
        assertEquals(
            2 * lapsAtCeiling,
            sm.spins(),
            "and it really bought the document a full budget again rather than refusing on " +
                "sight — the ceiling bounds a macrostep, it does not condemn a machine"
        )
    }

    /// The control: an ordinary document is untouched by any of this. Without
    /// it, an engine that refused every macrostep would pass the assertions
    /// above and fail nothing.
    @Test
    fun anOrdinaryMacrostepIsNotCounted() {
        val sm = started()

        deliver(sm, EventlessMacrostepIsBoundedEvent.Poke)

        assertEquals(1L, sm.pokes(), "the run fired")
        assertEquals(
            0,
            sm.truncatedMacrosteps(),
            "a macrostep of one microstep ends the way the clause says it does"
        )
        assertNull(sm.lastTruncatedMacrostepState())
        assertTrue(sm.currentState.value == EventlessMacrostepIsBoundedState.Idle)
    }
}
