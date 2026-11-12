// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 / 6.3: a late tick still honours `<cancel>`, and the clock is a seam the host can
// own. Swift AOT path.
//
// Two things the Kotlin driver asserts are not asserted here, and the reason is the language and
// not an oversight. `Engine.setClock` after `initialize` and `Engine.advanceTimeMs` on a clock the
// host cannot move are refused by `precondition`, which ends the process and cannot be caught by a
// test that goes on to report. The first is the engine's own rule (a deadline from one clock cannot
// be compared with one from another), and the second is stated in its documentation. The real-clock
// reading of "when is the next send due" is a range and not a number, so it is asked of a clock the
// test owns (`aHostOwnedClockAnswersExactly`), where the answer is exact.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import Foundation
import IntegrationMachines
import SceRuntime
import Testing

/// Past both `<send delay>`s in `waiting` (100 ms and 200 ms), with margin.
private let pastBothDeadlines: Int64 = 400

/// A clock that jumps forward on every reading.
///
/// This is what a descheduled host looks like from inside the engine: two readings taken for what
/// the document calls one instant come back different. A real one does it unpredictably and only
/// under load, which is why the defect it exposes reached a push before it reached a test; this one
/// does it on every reading, so the case is a verdict about the engine rather than about the
/// machine the suite runs on.
///
/// `stepMs` is 150 in the cases that use it, larger than the 100 ms separating the fixture's two
/// sends, which is exactly the condition under which per-statement readings reorder them.
private final class SteppingClock: SceClock {
    private let stepMs: Int64
    private var nowMs: Int64 = 0
    private(set) var readings = 0

    init(stepMs: Int64) {
        self.stepMs = stepMs
    }

    func elapsedMs() -> Int64 {
        readings += 1
        nowMs += stepMs
        return nowMs
    }
}

private typealias Machine = Engine<LateTickHonoursCancelPolicy>

private func started(on clock: any SceClock) -> Machine {
    let engine = Engine(policy: LateTickHonoursCancelPolicy())
    engine.setClock(clock)
    startRecordingARefusal(engine)
    return engine
}

private func milliseconds(_ duration: Duration) -> Int64 {
    let (seconds, attoseconds) = duration.components
    return seconds * 1000 + attoseconds / 1_000_000_000_000_000
}

/// Walks a host-owned clock by the engine's own answer, one deadline at a time, and says how many
/// steps it took.
private func driveByDeadlines(_ engine: Machine, limit: Int = 10) -> Int {
    var steps = 0
    while !engine.isInFinalState && steps < limit {
        steps += 1
        let next = engine.timeUntilNextScheduled() ?? .milliseconds(1)
        engine.advanceTimeMs(max(milliseconds(next), 1))
    }
    return steps
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct LateTickHonoursCancel {
    @Test func theFixtureIsSchedulerDriven() {
        let engine = started(on: ManualClock())
        #expect(
            engine.timeUntilNextScheduled() != nil,
            "the fixture arms two delayed <send>s; a machine with nothing scheduled means the document lost them, and every assertion below would then be measuring the wrong machine"
        )
    }

    @Test func aCancelSurvivesATickThatArrivesAfterBothDeadlines() {
        let engine = started(on: ManualClock())
        #expect(
            engine.getCurrentState() == .waiting,
            "the machine should be waiting on its two delayed sends")

        // The first turn after arming starts with both entries already due.
        engine.advanceTimeMs(pastBothDeadlines)

        #expect(
            engine.terminalState != .cancelLost,
            "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran first. Both entries were past due when this tick started, so the scheduler drain queued them together and the cancel found nothing left to drop. The clause cancels a send that has not been dispatched: dispatch is one entry per macrostep, not one queue-flush per tick."
        )

        // The verdict is itself scheduler-driven, so a tick loop that stopped working fails here
        // rather than passing by never moving.
        _ = driveByDeadlines(engine)
        #expect(
            engine.terminalState == .pass, "the machine did not reach `pass` after the cancel")
    }

    @Test func aPunctualHostReachesTheSameVerdict() {
        // The drive wakes the machine at each deadline in turn, so the first wake lands between
        // the two sends: the interleaving this case was written for, reached by construction. The
        // invariant is asserted on every wake-up rather than pinning `pass` alone, so a wrong
        // verdict names its cause.
        let engine = started(on: ManualClock())
        var steps = 0
        while !engine.isInFinalState && steps < 10 {
            steps += 1
            _ = driveByDeadlines(engine, limit: 1)
            #expect(
                engine.terminalState != .cancelLost,
                "whatever this loop's tick happened to straddle, `settle` must never be delivered after `active`'s <cancel sendid=\"s1\"> ran: the clause cancels a send that has not been dispatched, and dispatch is one entry per macrostep"
            )
        }
        #expect(
            engine.terminalState == .pass,
            "a host that keeps ticking must reach `pass`, whichever side of the two deadlines its wake-ups fell on"
        )
    }

    // MARK: - The clock as a seam: what a host that owns time gets

    @Test func aStallBetweenTwoArmsDoesNotReorderThem() {
        // `waiting`'s `<onentry>` arms `settle` at 200 ms and then `poke` at 100 ms, and the two
        // used to take a clock reading each. A host descheduled by more than the 100 ms between
        // them therefore gave `settle` the earlier deadline, the scheduler dispatched it first,
        // and the machine reached `cancelLost`: a verdict the document forbids, produced by nothing
        // the document says.
        //
        // Swept rather than pinned to one number, because 100, the gap between the two delays, is
        // where a per-statement reading flips the order, and a single case on one side of it
        // proves nothing about the other.
        for stallMs: Int64 in [1, 50, 99, 100, 101, 150, 1000] {
            let engine = started(on: SteppingClock(stepMs: stallMs))

            #expect(
                engine.getCurrentState() == .waiting,
                "stall \(stallMs)ms: the machine should still be waiting: arming is not delivery, however far the clock moved while it happened"
            )

            // Enough turns for the slowest stall in the sweep to cross both deadlines: at 1 ms a
            // turn, `poke` at 100 ms and `finish` 100 ms after that need roughly 200.
            var guardCount = 0
            while !engine.isInFinalState && guardCount < 1000 {
                guardCount += 1
                engine.tick()
                #expect(
                    engine.terminalState != .cancelLost,
                    "stall \(stallMs)ms: the clock moved that far between arming `settle` (200 ms) and arming `poke` (100 ms), and the engine gave `settle` the earlier deadline. Both sends are executed by one <onentry>, which is one microstep, an instant, so their deadlines must be 100 ms apart in the order their delays state."
                )
            }
            #expect(
                engine.terminalState == .pass,
                "stall \(stallMs)ms: a machine whose clock stalls must still reach `pass`: a stall changes when events arrive, never which ones do"
            )
        }
    }

    @Test func aFrozenClockDeliversNothingRatherThanEverything() {
        // The far end of the same sweep, and the one that says the engine reads the clock at all
        // rather than counting ticks. Time not moving is not an error condition: a simulation
        // paused at a breakpoint looks exactly like this, and the machine's answer must be that
        // nothing is due, not that everything is.
        let engine = started(on: SteppingClock(stepMs: 0))
        for _ in 0..<50 { engine.tick() }
        #expect(
            engine.getCurrentState() == .waiting,
            "no time passed, so neither of the two delayed sends came due, and 50 ticks must have delivered nothing"
        )
        #expect(
            engine.timeUntilNextScheduled() == .milliseconds(100),
            "a clock that has not moved leaves the nearer send exactly its full delay out")
    }

    @Test func aTickDispatchesWhatWasDueWhenItStarted() {
        // The other half of the same root cause, and the one that makes a slow tick unbounded
        // rather than merely late. A dispatch loop that re-read the clock on every pass would, under
        // this clock where every reading is 150 ms later than the last, run the whole document
        // (arm, dispatch, cancel, arm `finish`, dispatch it too) inside a single tick. One turn,
        // one reading: the first tick may deliver `poke` and must stop there, because `finish` was
        // armed during that same turn and is not due within it.
        let clock = SteppingClock(stepMs: 150)
        let engine = started(on: clock)
        let readingsBeforeTick = clock.readings

        engine.tick()
        let readingsInTick = clock.readings - readingsBeforeTick

        #expect(
            engine.getCurrentState() == .active,
            "one tick must dispatch what was due when it started and stop: this one ran on past `active`, and `finish` was armed during this very turn, so no reading taken inside the turn can find it due"
        )
        #expect(
            readingsInTick == 1,
            "a turn is one instant, so it takes one reading. \(readingsInTick) readings means the deadlines armed and judged inside this tick were measured against different instants"
        )
        #expect(
            readingsBeforeTick == 1,
            "initialize() is a turn too: entering the initial configuration arms both of this document's sends, and \(readingsBeforeTick) readings would mean they were measured from different instants"
        )

        engine.tick()
        #expect(
            engine.terminalState == .pass,
            "the next turn is a later instant, and `finish` is due in it")
    }

    @Test func aHostOwnedClockReachesTheSameVerdictWithoutSleeping() {
        // No sleep, so nothing here can be decided by the load on the build machine: this is the
        // case that makes the fixture's verdict a fact about the engine.
        let engine = started(on: ManualClock())

        #expect(engine.nowMs() == 0, "a host-owned clock starts where the host put it")
        #expect(
            engine.getCurrentState() == .waiting,
            "the machine should be waiting on its two delayed sends")

        // One coarse move, past BOTH deadlines: the condition the whole fixture exists for.
        engine.advanceTimeMs(250)

        #expect(
            engine.terminalState != .cancelLost,
            "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran first. Both entries were due when this move started; dispatch is one entry per macrostep, not one queue-flush per move"
        )
        #expect(
            engine.getCurrentState() == .active,
            "`poke` is the earlier deadline, so it is the one this move delivers")
        #expect(engine.nowMs() == 250, "the host moved time by 250 ms and by nothing else")

        // `finish` was armed at 250 + 100. Nothing is owed before then, and the engine says so
        // exactly rather than within a tolerance.
        #expect(
            engine.timeUntilNextScheduled() == .milliseconds(100),
            "`active` armed `finish` 100 ms out at the instant this move reached it")

        engine.advanceTimeMs(100)
        #expect(
            engine.terminalState == .pass,
            "moving exactly onto `finish`'s deadline must deliver it")
        #expect(
            engine.timeUntilNextScheduled() == nil,
            "nothing is scheduled once the machine is finished, so no clock movement is owed")
    }

    @Test func aHostOwnedClockAnswersExactly() {
        // The real-clock sibling of this question can only bound the answer, because arming
        // happens while the clock is running. Here the answer is a number, and a host stepping by
        // it lands on each deadline exactly, so no step can straddle two of them.
        let engine = started(on: ManualClock())

        #expect(
            engine.timeUntilNextScheduled() == .milliseconds(100),
            "the nearer of the two armed sends is 100 ms out, and on a clock the host owns that is exact"
        )

        let steps = driveByDeadlines(engine)

        #expect(
            engine.terminalState == .pass, "deadline-driven stepping did not reach `pass` in \(steps) steps")
        // 100 ms to `poke`, then 100 ms to `finish`: the run took exactly the time the document
        // asks for, with nothing spent guessing an interval.
        #expect(
            engine.nowMs() == 200,
            "a host driving by the engine's own answers spends exactly the document's delays and no more"
        )
    }

    @Test func oneGeneratedMachineServesBothKindsOfHost() throws {
        // The claim the seam exists to support, stated as one assertion: this is ONE generated
        // class, and which clock it runs on is a property of the instance.
        let onTheWall = Engine(policy: LateTickHonoursCancelPolicy())
        try onTheWall.initialize()
        let deadline = Date().addingTimeInterval(2)
        while !onTheWall.isInFinalState && Date() < deadline {
            onTheWall.tick()
            Thread.sleep(forTimeInterval: 0.01)
        }
        // The wall-clock half spends a 2 s budget on 200 ms of document time. Saying so separately
        // keeps a machine too loaded to finish from being reported as a machine that reached the
        // wrong configuration.
        #expect(
            onTheWall.isInFinalState,
            "the wall-clock half did not finish inside its 2000 ms budget, which is a statement about this host's load rather than about the engine"
        )

        let hostOwned = started(on: ManualClock())
        _ = driveByDeadlines(hostOwned)

        #expect(
            onTheWall.terminalState == hostOwned.terminalState,
            "the same generated machine must reach the same configuration whether the clock is the host's or the wall's: a clock decides WHEN events arrive and must never decide WHICH ones do"
        )
        #expect(
            hostOwned.terminalState == .pass, "and that configuration is the one the document specifies")
    }
}
