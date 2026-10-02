// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty — Kotlin AOT path.
//
// Every macrostep of such a machine ends, so the microstep ceiling never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: a host call that drains it did not return. The Kotlin
// runtime's `runMainEventLoop` had that shape until it took the budget
// ARCHITECTURE.md "External-Event Budget" states as one contract for every
// engine. This driver holds this engine to it: the same outcomes, the same
// arithmetic, the same document as the Python, Rust and Go ones.
//
// The delayed outcomes run on ManualClock, so nothing here sleeps and the clock
// moves only where a case moves it. This engine hands a STATIC zero delay to its
// scheduler (Rust reads it as undelayed), so `zero` and `zero_expr` both reach
// the same-instant bound here. Synchronous mode only: the coroutine mode never
// returns to the host, so there is no call to hand back.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// integration_resources/ for the reason
// scripts/regen_external_chain_is_bounded.sh states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_external_chain_is_bounded_kotlin.sh

package com.sce.integration

import com.sce.integration.external_chain_is_bounded.ExternalChainIsBoundedEvent
import com.sce.integration.external_chain_is_bounded.ExternalChainIsBoundedStateMachine
import com.sce.runtime.ManualClock
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ExternalChainIsBounded — External-Event Budget")
class ExternalChainIsBoundedTest {

    // The default the contract states, spelled here rather than read back from
    // the engine: a test that asked the engine for its own limit would agree
    // with any limit, including one an edit moved by three orders of magnitude.
    private val defaultBudget = 10_000

    /// The machine on host-owned time. The clock is installed BEFORE
    /// `initialize()`: the engine refuses a clock afterwards, because deadlines
    /// armed against one do not compare with another.
    private fun started(): ExternalChainIsBoundedStateMachine {
        val sm = ExternalChainIsBoundedStateMachine(W3CTestBase.createEngine())
        sm.clock = ManualClock(0L)
        sm.initialize()
        return sm
    }

    /// One host call that delivers `event`: the engine takes it off the queue it
    /// was submitted to, as the first event of the invocation that runs.
    private fun deliver(sm: ExternalChainIsBoundedStateMachine, event: ExternalChainIsBoundedEvent) {
        sm.send(event)
        sm.tick()
    }

    @Test
    fun theDefaultBudgetIsTheDocumentedOne() {
        val sm = started()
        assertEquals(defaultBudget, sm.maxExternalEventsPerCall())
        assertEquals(0, sm.truncatedEventChains())
        assertNull(sm.lastTruncatedEvent())
    }

    // This test returning at all is half the assertion: before the budget the
    // call did not.
    @Test
    fun aChainThatCannotEndIsCutAtTheBudgetAndTheCallReturns() {
        val sm = started()

        deliver(sm, ExternalChainIsBoundedEvent.Spin)

        assertEquals(
            1,
            sm.truncatedEventChains(),
            "the call handed control back with an event still queued, and said so; without the count " +
                "the host sees a machine that is running and has returned, with no sign that anything " +
                "went wrong",
        )
        // The host's own event is the first of the invocation, so the budget buys
        // the host's event and then `budget - 1` links.
        assertEquals(
            (defaultBudget - 1).toLong(),
            sm.links(),
            "the chain must run exactly as far as the budget allows: fewer means the call was cut " +
                "early, more means the budget moved",
        )
        assertEquals(
            ExternalChainIsBoundedEvent.Link,
            sm.lastTruncatedEvent(),
            "the count says a call did not reach quiet; this says what it was still taking",
        )
    }

    // The half that makes the count mean something: a chain that ends on its own
    // is not refused, however close to the budget it comes. `bounded` is the
    // host's event and five laps, six in all.
    @Test
    fun theBudgetIsExactForAChainThatEndsByItself() {
        val exactly = started()
        exactly.setMaxExternalEventsPerCall(6)
        deliver(exactly, ExternalChainIsBoundedEvent.Bounded)
        assertEquals(5L, exactly.laps())
        assertEquals(
            0,
            exactly.truncatedEventChains(),
            "a call that takes exactly the budget and empties the queue refused nothing: a long " +
                "chain is not a runaway",
        )
        assertNull(exactly.lastTruncatedEvent())

        val oneShort = started()
        oneShort.setMaxExternalEventsPerCall(5)
        deliver(oneShort, ExternalChainIsBoundedEvent.Bounded)
        assertEquals(4L, oneShort.laps(), "one lap was left queued")
        assertEquals(1, oneShort.truncatedEventChains())
        assertEquals(ExternalChainIsBoundedEvent.Lap, oneShort.lastTruncatedEvent())
    }

    // What the refusal did with the events it would not take: it left them
    // queued. An engine that dropped the queue stops short and never finishes;
    // one that ran the chain anyway finishes it in the first call.
    @Test
    fun aRefusedCallLeavesTheQueueSoTheNextCallFinishesTheChain() {
        val sm = started()
        sm.setMaxExternalEventsPerCall(20)

        deliver(sm, ExternalChainIsBoundedEvent.Resume)
        assertEquals(1, sm.truncatedEventChains())
        assertEquals(19L, sm.beats(), "the host's event and nineteen beats")

        deliver(sm, ExternalChainIsBoundedEvent.Poke)
        assertEquals(
            30L,
            sm.beats(),
            "the second call took the beats the first left on the queue, each in a budget of its " +
                "own, and finished",
        )
        assertEquals(1L, sm.pokes(), "and the host's second event was heard")
        assertEquals(
            1,
            sm.truncatedEventChains(),
            "the second call ended the way the clause says: nothing more is counted",
        )
    }

    // delay="0ms" is due at the instant being processed. This engine hands it to
    // its scheduler, so a tick pops the entry, its handler arms another due at
    // the same reading, and the tick that is popping finds it. Each pass takes
    // one event, so a budget on the drain alone never trips — this is the case
    // ARCHITECTURE.md rule 5 exists for. This test returning at all is the
    // assertion.
    @Test
    fun aChainThroughAStaticDelayOfZeroDoesNotKeepTheTickFromReturning() {
        val sm = started()
        sm.setMaxExternalEventsPerCall(50)

        deliver(sm, ExternalChainIsBoundedEvent.Zero)
        assertEquals(
            0,
            sm.truncatedEventChains(),
            "entering the state arms one entry and pops none: nothing has been cut yet",
        )

        sm.tick()

        assertEquals(
            1,
            sm.truncatedEventChains(),
            "the tick popped entries due at its own reading until the budget, left the due one " +
                "waiting and said so",
        )
        assertEquals(50L, sm.blinks(), "the budget of pops at one reading, no more and no fewer")
        assertEquals(ExternalChainIsBoundedEvent.Blink, sm.lastTruncatedEvent())
    }

    // The same chain through delayexpr="'0ms'", which has no static value an
    // engine could read as undelayed.
    @Test
    fun aChainThroughADelayExpressionThatIsZeroDoesNotKeepTheTickFromReturning() {
        val sm = started()
        sm.setMaxExternalEventsPerCall(50)

        deliver(sm, ExternalChainIsBoundedEvent.ZeroExpr)
        sm.tick()

        assertEquals(1, sm.truncatedEventChains())
        assertEquals(50L, sm.exprs(), "the budget of pops at one reading, no more and no fewer")
    }

    // Eight pulses, each due one millisecond after the last. They are due at
    // later instants, so a legitimate time-driven workload is not a runaway
    // however small the budget: three here, against eight events.
    @Test
    fun aChainThatIsFiniteBecauseTheClockIsIsNotRefused() {
        val walked = started()
        walked.setMaxExternalEventsPerCall(3)
        deliver(walked, ExternalChainIsBoundedEvent.Timed)
        repeat(8) { walked.advanceTimeMs(1) }
        assertEquals(8L, walked.pulses())
        assertEquals(
            0,
            walked.truncatedEventChains(),
            "each pulse came in a tick of its own, at an instant of its own",
        )

        val jumped = started()
        jumped.setMaxExternalEventsPerCall(3)
        deliver(jumped, ExternalChainIsBoundedEvent.Timed)
        jumped.advanceTimeMs(8)
        assertEquals(8L, jumped.pulses())
        assertEquals(
            0,
            jumped.truncatedEventChains(),
            "eight entries came due on the way to one reading, seven of them at earlier instants: " +
                "a clock that moved a long way is bounded by how far it moved, and is not a chain " +
                "that does not end",
        )
    }

    @Test
    fun aHostChoosesTheBudgetAndABudgetThatTakesNoEventIsRefused() {
        val sm = started()
        sm.setMaxExternalEventsPerCall(7)
        assertEquals(7, sm.maxExternalEventsPerCall())
        for (refused in listOf(0, -1, -100)) {
            val error = assertThrows(IllegalArgumentException::class.java) {
                sm.setMaxExternalEventsPerCall(refused)
            }
            assertTrue(error.message!!.contains("at least one"), "the refusal says why: ${error.message}")
        }
        assertEquals(7, sm.maxExternalEventsPerCall(), "a refused budget changes nothing")
    }
}
