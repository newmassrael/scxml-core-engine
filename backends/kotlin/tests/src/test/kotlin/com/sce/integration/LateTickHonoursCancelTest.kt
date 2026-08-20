// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.2 + §scxml-6.3: a `<cancel>` still lands when the host ticked late —
// Kotlin AOT path (sync mode).
//
// In sync mode the scheduled sends live in a time-ordered queue that `tick()`
// drains. Draining it to exhaustion before running a macrostep is the defect:
// a host that wakes after two fire times have passed holds both entries, and
// queueing both makes the second undroppable before the first one's
// transitions have run. The `<cancel>` then executes against a queue the event
// has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A
// host that wakes between them passes either way, which is why every existing
// suite was blind to this.
//
// This channel is the one that reads a real clock, and that made a second
// defect visible here and nowhere else: the two `<send>`s of one `<onentry>`
// used to take a clock reading each, so a host descheduled between them
// computed their deadlines against two different instants. The stalling-clock
// cases below are that defect's witness, and they are deterministic — the
// clock is the seam, so the stall is injected rather than waited for.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_late_tick_honours_cancel_kotlin.sh

package com.sce.integration

import com.sce.integration.late_tick_honours_cancel.LateTickHonoursCancelState
import com.sce.integration.late_tick_honours_cancel.LateTickHonoursCancelStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.MonotonicClock
import com.sce.runtime.SceClock
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// §scxml-6.2/§scxml-6.3 — a late tick still honours `<cancel>` (Kotlin AOT).
@DisplayName("LateTickHonoursCancel — W3C SCXML 6.2/6.3")
class LateTickHonoursCancelTest {

    /** Past both `<send delay>`s in `waiting` (100 ms and 200 ms), with margin. */
    private val pastBothDeadlines = 400L

    /**
     * A clock that jumps forward on every reading.
     *
     * This is what a descheduled host looks like from inside the engine: two
     * readings taken for what the document calls one instant come back
     * different. A real one does it unpredictably and only under load, which
     * is why the defect it exposes reached a push before it reached a test;
     * this one does it on every reading, so the case below is a verdict about
     * the engine rather than about the machine the suite runs on.
     *
     * [stepMs] is 150 in the cases that use it — larger than the 100 ms
     * separating the fixture's two sends, which is exactly the condition under
     * which per-statement readings reorder them.
     */
    private class SteppingClock(private val stepMs: Long) : SceClock {
        private var nowMs = 0L
        var readings = 0
            private set

        override fun elapsedMs(): Long {
            readings++
            nowMs += stepMs
            return nowMs
        }
    }

    private fun started(): LateTickHonoursCancelStateMachine {
        val sm = LateTickHonoursCancelStateMachine()
        sm.initialize()
        return sm
    }

    private fun startedOn(clock: SceClock): LateTickHonoursCancelStateMachine {
        val sm = LateTickHonoursCancelStateMachine()
        sm.clock = clock
        sm.initialize()
        return sm
    }

    @Test
    fun theFixtureIsSchedulerDriven() {
        assertTrue(
            started().needsEventScheduler,
            "the fixture arms two delayed <send>s; a machine that does not declare " +
                "needsEventScheduler means the document lost them, and every assertion " +
                "below would then be measuring the wrong machine"
        )
    }

    @Test
    fun aCancelSurvivesATickThatArrivesAfterBothDeadlines() {
        val sm = started()
        assertEquals(
            LateTickHonoursCancelState.Waiting,
            sm.currentState.value,
            "the machine should be waiting on its two delayed sends"
        )

        val armedFor = System.currentTimeMillis()
        Thread.sleep(pastBothDeadlines)
        val sleptFor = System.currentTimeMillis() - armedFor
        sm.tick()
        val tickTook = System.currentTimeMillis() - armedFor - sleptFor

        assertNotEquals(
            LateTickHonoursCancelState.CancelLost,
            sm.currentState.value,
            "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran " +
                "first. Both entries were past due when this tick started, so the " +
                "scheduler drain queued them together and the cancel found nothing " +
                "left to drop. W3C SCXML 6.3 cancels a send that has not been " +
                "dispatched — dispatch is one entry per macrostep, not one " +
                "queue-flush per tick. " +
                // This engine reads a real clock. The two numbers separate the
                // explanations a wall-clock failure has: a slept-for far above
                // 400 ms means the host was descheduled before the tick, while a
                // tick that took long enough to cross a deadline of its own means
                // the dispatch loop chased a deadline it created. Neither can
                // reorder the two arms any more — `aStallBetweenTwoArmsDoesNotReorderThem`
                // owns that, deterministically — so a failure here is a new fact.
                "[slept ${sleptFor}ms, tick took ${tickTook}ms]"
        )

        // The verdict is itself scheduler-driven, so a channel whose tick loop
        // stopped working fails here rather than passing by never moving.
        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(20L)
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "the machine did not reach `pass` after the cancel"
        )
        sm.cleanup()
    }

    @Test
    fun aPunctualHostReachesTheSameVerdict() {
        // A 10 ms tick loop USUALLY wakes between the two deadlines, which is the
        // interleaving this case was written for — but nothing makes it do so on
        // a loaded host, where the first tick can land past both. That is the
        // sibling case above, and the two must reach the same verdict: the name
        // of this test is the contract, not the schedule it hoped for.
        //
        // Measured 2026-08-19: pinning `Pass` alone let a late first tick report
        // `CancelLost` as "the punctual host failed", which is a false red about
        // a real defect the other case already owns. Asserting the invariant on
        // every wake-up is both stronger and timing-free.
        val sm = started()
        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            assertNotEquals(
                LateTickHonoursCancelState.CancelLost,
                sm.currentState.value,
                "whatever this loop's tick happened to straddle, `settle` must never " +
                    "be delivered after `active`'s <cancel sendid=\"s1\"> ran. W3C SCXML " +
                    "6.3 cancels a send that has not been dispatched, and dispatch is " +
                    "one entry per macrostep"
            )
            Thread.sleep(10L)
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "a host that keeps ticking must reach `pass`, whichever side of the two " +
                "deadlines its wake-ups fell on"
        )
        sm.cleanup()
    }

    @Test
    fun theEngineSaysWhenItIsNextDue() {
        // This engine reads a real clock, so the question "is anything due yet"
        // is only answerable relative to how long the run itself took. Arming
        // happens somewhere inside `started()`, so `elapsed` below is an UPPER
        // bound on the time that has passed since the nearer send was armed.
        //
        // Measured 2026-08-19: asserting `due > 0` outright is a race with the
        // machine this suite runs on. It held on an idle host and failed inside
        // the 27-gate push, where `started()` — SCXML parse plus a script-engine
        // session — can itself outlast the 100 ms deadline. That is a false red:
        // the engine answered correctly about a send that really was due.
        // `aHostOwnedClockAnswersExactly` is the same question asked of a clock
        // the test owns, where the answer is a number rather than a range.
        val before = System.currentTimeMillis()
        val sm = started()
        val due = sm.timeUntilNextScheduledMs()
        val elapsed = System.currentTimeMillis() - before

        assertTrue(
            due != null && due <= 100L,
            "the nearer of the two armed sends is 100 ms out; the engine answered " +
                "$due, which would send a host past the earlier deadline"
        )
        // The lower bound is the half that catches an answer of "due now", which
        // reads as a working query and costs the caller a spin that never sleeps.
        // Stated against the clock this run actually observed: the send was armed
        // no earlier than `before`, so at least `100 - elapsed` ms of it remain.
        assertTrue(
            due != null && due >= 100L - elapsed,
            "the nearer send was armed at most $elapsed ms ago, so at least " +
                "${100L - elapsed} ms of its 100 ms delay are left, but the engine " +
                "answered $due. A host sleeping on that answer does not sleep long enough"
        )

        // Drive by the engine's own answer: every wait lands on a fire time, so
        // no wait can straddle two of them.
        val deadline = System.currentTimeMillis() + 3000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            val wait = sm.timeUntilNextScheduledMs() ?: 5L
            Thread.sleep(maxOf(wait, 1L))
            sm.tick()
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "deadline-driven ticking did not reach `pass`"
        )
        assertNull(
            sm.timeUntilNextScheduledMs(),
            "nothing is scheduled once the machine is finished, so no wake-up is owed"
        )
        sm.cleanup()
    }

    // --- The clock as a seam: what a host that owns time gets ---

    @Test
    fun aStallBetweenTwoArmsDoesNotReorderThem() {
        // The witness for the defect this channel had and the virtual-clock
        // channels could not have. `waiting`'s `<onentry>` arms `settle` at
        // 200 ms and then `poke` at 100 ms, and the two used to take a clock
        // reading each. A host descheduled by more than the 100 ms between them
        // therefore gave `settle` the earlier deadline, the scheduler dispatched
        // it first, and the machine reached `cancelLost` — a verdict the
        // document forbids, produced by nothing the document says.
        //
        // Swept rather than pinned to one number, because 100 — the gap between
        // the two delays — is where a per-statement reading flips the order, and
        // a single case on one side of it proves nothing about the other. A
        // stall of 99 ms used to pass and 100 ms used to fail, which is what a
        // defect decided by the host's scheduler looks like from the outside:
        // intermittent. The verdict must be the same for every one of these.
        for (stallMs in longArrayOf(1L, 50L, 99L, 100L, 101L, 150L, 1000L)) {
            val sm = startedOn(SteppingClock(stepMs = stallMs))

            assertEquals(
                LateTickHonoursCancelState.Waiting,
                sm.currentState.value,
                "stall ${stallMs}ms: the machine should still be waiting — arming is " +
                    "not delivery, however far the clock moved while it happened"
            )

            // Enough turns for the slowest stall in the sweep to cross both
            // deadlines: at 1 ms a turn, `poke` at 100 ms and `finish` 100 ms
            // after that need roughly 200.
            var guard = 0
            while (!sm.isInFinalState && guard++ < 1000) {
                sm.tick()
                assertNotEquals(
                    LateTickHonoursCancelState.CancelLost,
                    sm.currentState.value,
                    "stall ${stallMs}ms: the clock moved that far between arming " +
                        "`settle` (200 ms) and arming `poke` (100 ms), and the engine " +
                        "gave `settle` the earlier deadline. Both sends are executed by " +
                        "one <onentry>, which is one microstep — an instant, per W3C " +
                        "SCXML 3.13 — so their deadlines must be 100 ms apart in the " +
                        "order their delays state. Reading the clock once per <send> " +
                        "instead of once per turn makes the host's scheduling decide " +
                        "which of two events the document ordered arrives first"
                )
            }
            assertEquals(
                LateTickHonoursCancelState.Pass,
                sm.currentState.value,
                "stall ${stallMs}ms: a machine whose clock stalls must still reach " +
                    "`pass` — a stall changes when events arrive, never which ones do"
            )
            sm.cleanup()
        }
    }

    @Test
    fun aFrozenClockDeliversNothingRatherThanEverything() {
        // The far end of the same sweep, and the one that says the engine reads
        // the clock at all rather than counting ticks. Time not moving is not
        // an error condition — a simulation paused at a breakpoint looks exactly
        // like this — and the machine's answer must be that nothing is due, not
        // that everything is.
        val sm = startedOn(SteppingClock(stepMs = 0L))
        repeat(50) { sm.tick() }
        assertEquals(
            LateTickHonoursCancelState.Waiting,
            sm.currentState.value,
            "no time passed, so neither of the two delayed sends came due, and 50 " +
                "ticks must have delivered nothing"
        )
        assertEquals(
            100L,
            sm.timeUntilNextScheduledMs(),
            "a clock that has not moved leaves the nearer send exactly its full delay out"
        )
        sm.cleanup()
    }

    @Test
    fun aTickDispatchesWhatWasDueWhenItStarted() {
        // The other half of the same root cause, and the one that makes a slow
        // tick unbounded rather than merely late. The dispatch loop used to
        // re-read the clock on every pass, so a tick that took long enough to
        // cross the next deadline dispatched that entry too, and then the one
        // after it. The engine was then chasing deadlines its own slowness had
        // created, in a loop the host cannot get between.
        //
        // Under this clock every reading is 150 ms later than the last, so a
        // loop that re-read it would run the whole document — arm at 150/300,
        // dispatch, cancel, arm `finish`, dispatch it too — inside a single
        // tick. One turn, one reading: the first tick may deliver `poke` and
        // must stop there, because `finish` was armed during that same turn and
        // is not due within it.
        val clock = SteppingClock(stepMs = 150L)
        val sm = startedOn(clock)
        val readingsBeforeTick = clock.readings

        sm.tick()
        val readingsInTick = clock.readings - readingsBeforeTick

        assertEquals(
            LateTickHonoursCancelState.Active,
            sm.currentState.value,
            "one tick must dispatch what was due when it started and stop. This one " +
                "ran on past `active` — `finish` was armed during this very turn, so " +
                "no reading taken inside the turn can find it due. A dispatch loop " +
                "that re-reads a clock which moves while it runs never has to give " +
                "the host its thread back"
        )
        assertEquals(
            1,
            readingsInTick,
            "a turn is one instant, so it takes one reading. $readingsInTick readings " +
                "means the deadlines armed and judged inside this tick were measured " +
                "against $readingsInTick different instants, and which of them a given " +
                "entry was compared against depends on how far the loop had got"
        )
        assertEquals(
            1,
            readingsBeforeTick,
            "initialize() is a turn too: entering the initial configuration arms both " +
                "of this document's sends, and $readingsBeforeTick readings would mean " +
                "they were measured from different instants"
        )

        sm.tick()
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "the next turn is a later instant, and `finish` is due in it"
        )
        sm.cleanup()
    }

    @Test
    fun aHostOwnedClockReachesTheSameVerdictWithoutSleeping() {
        // The contract the Python channel already answers — `advance_time(ms)`
        // moves the clock and runs whatever that made due — spelled on a
        // backend whose default clock is the wall. That is the part no
        // reference in this repository offers: Python owns time and cannot do
        // otherwise, the Rust HAL is chosen when the machine is compiled, and
        // this one is a property of the instance, so the same generated machine
        // serves a production host on the wall clock and a simulation that
        // owns time outright.
        //
        // No sleep, so nothing here can be decided by the load on the build
        // machine: this is the case that makes `LateTickHonoursCancel`'s
        // verdict a fact about the engine.
        val sm = LateTickHonoursCancelStateMachine()
        sm.clock = ManualClock()
        sm.initialize()

        assertEquals(0L, sm.nowMs(), "a host-owned clock starts where the host put it")
        assertEquals(
            LateTickHonoursCancelState.Waiting,
            sm.currentState.value,
            "the machine should be waiting on its two delayed sends"
        )

        // One coarse move, past BOTH deadlines — the condition the whole
        // fixture exists for, and here it is a number rather than a hope.
        sm.advanceTimeMs(250L)

        assertNotEquals(
            LateTickHonoursCancelState.CancelLost,
            sm.currentState.value,
            "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran " +
                "first. Both entries were due when this move started; dispatch is one " +
                "entry per macrostep, not one queue-flush per move"
        )
        assertEquals(
            LateTickHonoursCancelState.Active,
            sm.currentState.value,
            "`poke` is the earlier deadline, so it is the one this move delivers"
        )
        assertEquals(250L, sm.nowMs(), "the host moved time by 250 ms and by nothing else")

        // `finish` was armed at 250 + 100. Nothing is owed before then, and the
        // engine says so exactly rather than within a tolerance.
        assertEquals(
            100L,
            sm.timeUntilNextScheduledMs(),
            "`active` armed `finish` 100 ms out at the instant this move reached it"
        )

        sm.advanceTimeMs(100L)
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "moving exactly onto `finish`'s deadline must deliver it"
        )
        assertNull(
            sm.timeUntilNextScheduledMs(),
            "nothing is scheduled once the machine is finished, so no clock movement is owed"
        )
        sm.cleanup()
    }

    @Test
    fun aHostOwnedClockAnswersExactly() {
        // The real-clock sibling of this question (`theEngineSaysWhenItIsNextDue`)
        // can only bound the answer, because arming happens while the clock is
        // running. Here the answer is a number, and a host stepping by it lands
        // on each deadline exactly — so no step can straddle two of them.
        val sm = LateTickHonoursCancelStateMachine()
        sm.clock = ManualClock()
        sm.initialize()

        assertEquals(
            100L,
            sm.timeUntilNextScheduledMs(),
            "the nearer of the two armed sends is 100 ms out, and on a clock the host " +
                "owns that is exact"
        )

        var steps = 0
        while (!sm.isInFinalState && steps++ < 10) {
            val step = sm.timeUntilNextScheduledMs()
            assertTrue(
                step != null,
                "the machine is not finished and nothing is scheduled, so a host driving " +
                    "by deadlines alone would stall here"
            )
            sm.advanceTimeMs(maxOf(step ?: 0L, 1L))
        }

        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "deadline-driven stepping did not reach `pass`"
        )
        // 100 ms to `poke`, then 100 ms to `finish`: the run took exactly the
        // time the document asks for, with nothing spent guessing an interval.
        assertEquals(
            200L,
            sm.nowMs(),
            "a host driving by the engine's own answers spends exactly the document's " +
                "delays and no more"
        )
        sm.cleanup()
    }

    @Test
    fun oneGeneratedMachineServesBothKindsOfHost() {
        // The claim the seam exists to support, stated as one assertion rather
        // than left implicit across the cases above: this is ONE generated
        // class, and which clock it runs on is a property of the instance.
        //
        // Neither reference in this repository can say that. The Python
        // channel owns time and cannot do otherwise, so a Python host wanting
        // wall-clock behaviour writes its own loop; the Rust channel binds its
        // clock into the policy type at generation time, so a machine
        // generated for the wall cannot be handed a synthetic clock without
        // being generated again. Measured 2026-08-20 with a stepping `Hal`:
        // the Rust engine also arms each `<send>` against its own reading, so
        // a 1500 ms stall reorders a 1000 ms gap there — the seam alone is not
        // the fix, and the reference has the seam.
        val onTheWall = started()
        val deadline = System.currentTimeMillis() + 2000L
        while (!onTheWall.isInFinalState && System.currentTimeMillis() < deadline) {
            onTheWall.tick()
            Thread.sleep(10L)
        }
        // The wall-clock half spends a 2 s budget on 200 ms of document time.
        // Saying so separately keeps a machine too loaded to finish from being
        // reported as a machine that reached the wrong configuration — the
        // distinction this whole round is about.
        assertTrue(
            onTheWall.isInFinalState,
            "the wall-clock half did not finish inside its 2000 ms budget, which is a " +
                "statement about this host's load rather than about the engine. The " +
                "document needs 200 ms of it"
        )

        val hostOwned = LateTickHonoursCancelStateMachine()
        hostOwned.clock = ManualClock()
        hostOwned.initialize()
        var steps = 0
        while (!hostOwned.isInFinalState && steps++ < 10) {
            hostOwned.advanceTimeMs(maxOf(hostOwned.timeUntilNextScheduledMs() ?: 1L, 1L))
        }

        assertEquals(
            onTheWall.currentState.value,
            hostOwned.currentState.value,
            "the same generated machine must reach the same configuration whether the " +
                "clock is the host's or the wall's — a clock decides WHEN events arrive " +
                "and must never decide WHICH ones do"
        )
        assertEquals(
            LateTickHonoursCancelState.Pass,
            hostOwned.currentState.value,
            "and that configuration is the one the document specifies"
        )
        onTheWall.cleanup()
        hostOwned.cleanup()
    }

    @Test
    fun theClockCannotBeSwappedOnceTheMachineHasArmedAgainstIt() {
        // The seam has one ordering rule, and it is load-bearing rather than
        // decorative: `initialize()` runs the entry configuration's `<onentry>`,
        // which arms both of this document's sends. Deadlines computed against
        // one clock and compared against another are not comparable at all, so
        // the engine refuses instead of producing a schedule nobody chose.
        val sm = started()
        assertThrows(IllegalStateException::class.java) {
            sm.clock = ManualClock()
        }
        sm.cleanup()
    }

    @Test
    fun advanceTimeRefusesAClockTheHostDoesNotOwn() {
        // The failure mode this rejects is a silent one: a host that calls
        // `advanceTimeMs` on a wall-clock engine believes it decided when the
        // events arrive, and they arrive on a schedule it has no control over.
        // A machine that looks deterministic and is not is worse than one that
        // says so.
        val sm = LateTickHonoursCancelStateMachine()
        sm.clock = MonotonicClock()
        sm.initialize()
        assertThrows(IllegalStateException::class.java) {
            sm.advanceTimeMs(250L)
        }
        sm.cleanup()
    }
}
