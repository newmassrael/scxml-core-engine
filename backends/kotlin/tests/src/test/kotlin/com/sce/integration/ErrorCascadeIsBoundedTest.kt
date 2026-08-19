// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises `error.execution`,
// the same transition answers it, and the drain never empties. Kotlin AOT path.
//
// This engine did not spin — `drainEventlessAndInternal` stops after a hundred
// iterations — and that is the sharper half of the finding: bounded and silent
// reads to the host exactly like unbounded. The chain was cut and nothing said
// so, so a supervisor watching a machine whose error handling is broken saw a
// machine that had gone quiet.
//
// The fixture separates a chain that STOPS by itself (`settle`, three links,
// then its guard stops matching) from one that cannot (`spin`). Both are runs
// of errors, and only the second is a defect.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_error_cascade_is_bounded_kotlin.sh

package com.sce.integration

import com.sce.integration.error_cascade_is_bounded.ErrorCascadeIsBoundedEvent
import com.sce.integration.error_cascade_is_bounded.ErrorCascadeIsBoundedState
import com.sce.integration.error_cascade_is_bounded.ErrorCascadeIsBoundedStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.12.2 — a handler that answers its own failure with the same
/// failure is stopped, and the host is told.
@DisplayName("ErrorCascadeIsBounded — W3C SCXML 3.12.2")
class ErrorCascadeIsBoundedTest {

    /** The ceiling the engine applies, spelled here rather than read back from
     * it. A test that asked the engine for its own limit would agree with any
     * limit, including one an edit moved by three orders of magnitude. */
    private val maxLinks = 100L

    private fun started(): ErrorCascadeIsBoundedStateMachine {
        // The fixture counts handler runs with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = ErrorCascadeIsBoundedStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: ErrorCascadeIsBoundedStateMachine, event: ErrorCascadeIsBoundedEvent) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: the chain is cut at a known link, and the count says it was.
    @Test
    fun aHandlerThatCannotHandleItsErrorIsStopped() {
        val sm = started()
        assertEquals(0, sm.errorCascadeEvents(), "nothing has been refused before the machine has done anything")

        deliver(sm, ErrorCascadeIsBoundedEvent.Spin)

        assertEquals(
            maxLinks,
            sm.runs(),
            "`runaway`'s handler must run exactly as many times as the engine allows links " +
                "in a chain — fewer means the document was cut off early, more means the " +
                "ceiling moved"
        )
        assertEquals(
            1,
            sm.errorCascadeEvents(),
            "the handler's <assign> failed again on the last allowed link, and the error it " +
                "raised is the one the engine refused to queue. Stopping at an iteration " +
                "ceiling without this count is what left the host reading a quiet machine"
        )
        assertEquals(
            ErrorCascadeIsBoundedEvent.Error.Execution,
            sm.lastErrorCascadeEvent(),
            "a count alone does not name the repair: error.execution is a handler whose own " +
                "content fails, error.communication one that answers an unreachable target " +
                "by talking to it again"
        )
        assertEquals(
            ErrorCascadeIsBoundedState.Runaway,
            sm.currentState.value,
            "the handler is targetless, so nothing here may move the machine"
        )
    }

    /// The other half, and the one that makes the count mean something: a
    /// chain that ends by itself must pass through untouched.
    @Test
    fun aChainThatEndsOnItsOwnIsNotRefused() {
        val sm = started()

        deliver(sm, ErrorCascadeIsBoundedEvent.Settle)

        assertEquals(
            3L,
            sm.repairs(),
            "`settling`'s handler repairs three times and then its `repairs < 3` guard stops " +
                "matching. Three links is what a real repair strategy looks like, and the " +
                "engine must not have interrupted it"
        )
        assertEquals(
            0,
            sm.errorCascadeEvents(),
            "nothing was refused: the chain ended on the document's own terms. A ceiling that " +
                "fired here would report every document that fails often as one that cannot " +
                "stop failing"
        )
        assertNull(sm.lastErrorCascadeEvent(), "nothing was refused, so there is no last one to name")
        assertEquals(
            1,
            sm.unhandledErrorEvents(),
            "the fourth error found no matching transition once the guard closed, which is the " +
                "ordinary clause — the two counts answer different questions and this document " +
                "produces exactly one of each"
        )
    }

    /// A single failure with nobody to answer it is not a chain.
    @Test
    fun oneErrorNobodyAnsweredIsNotAChain() {
        val sm = started()

        repeat(5) { deliver(sm, ErrorCascadeIsBoundedEvent.Boom) }

        assertEquals(5, sm.unhandledErrorEvents(), "five failures, none of them answered — the clause's own case")
        assertEquals(
            0,
            sm.errorCascadeEvents(),
            "no handler ran, so no handler raised anything: a count keyed off how OFTEN a " +
                "document fails would already be at five here"
        )
    }

    /// Cutting the chain must not cost the document the states that work.
    @Test
    fun theMachineStillAnswersAfterItsChainIsCut() {
        val sm = started()

        deliver(sm, ErrorCascadeIsBoundedEvent.Spin)
        assertEquals(1, sm.errorCascadeEvents(), "precondition: this test is about what happens AFTER a refusal")

        deliver(sm, ErrorCascadeIsBoundedEvent.Poke)

        assertEquals(
            1L,
            sm.pokes(),
            "`runaway` answers `poke` with a targetless transition, and it ran — an engine that " +
                "stopped the machine to end the chain would leave the host with a dead document " +
                "instead of a bounded one"
        )
        assertEquals(
            1,
            sm.errorCascadeEvents(),
            "`poke` raises nothing, so the count that was already there is all there is: the " +
                "refusal is a fact about the past, not a mode"
        )
    }

    /// The depth is a property of the chain, not of the machine's whole life.
    @Test
    fun aSecondChainStartsFromZero() {
        val sm = started()

        deliver(sm, ErrorCascadeIsBoundedEvent.Spin)
        deliver(sm, ErrorCascadeIsBoundedEvent.Reset)
        assertTrue(
            sm.currentState.value == ErrorCascadeIsBoundedState.Idle,
            "`reset` is the fixture's way back out of the chain"
        )

        deliver(sm, ErrorCascadeIsBoundedEvent.Spin)

        assertEquals(
            2 * maxLinks,
            sm.runs(),
            "the second entry into `runaway` must buy the document a full chain again. A depth " +
                "carried across the drains would stop this one at its first link and leave the " +
                "counter at $maxLinks"
        )
        assertEquals(
            2,
            sm.errorCascadeEvents(),
            "two chains, two refusals — a count that saturates at one would read as a machine " +
                "that recovered"
        )
    }
}
