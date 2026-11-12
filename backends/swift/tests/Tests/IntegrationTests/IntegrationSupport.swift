// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand, and shared by the drivers of this directory that start a child session: what a
// driver means by "the run reached it" is said once here and not once per stem.

import Foundation
import SceLua
import SceRuntime
import Testing

/// What a run left in the variables of an ECMAScript-datamodel machine that is not the value the
/// document's comment says it should be, each as `name=<read> (wanted <n>)`. Empty when every
/// variable holds its integer: a driver asserts the list is empty, so a failure names every
/// variable that is wrong and not only the first.
func mismatches(
    _ expected: [(name: String, want: Int64)], reading read: (String) -> LuaValue?
) -> [String] {
    expected.filter { read($0.name) != .integer($0.want) }
        .map { "\($0.name)=\(String(describing: read($0.name))) (wanted \($0.want))" }
}

/// Lets the machine tick, for at most `timeout`, until `reached` holds, and says whether it did.
///
/// A machine that invokes a child session, or sends itself a delayed event, is only moved on by the
/// ticks: the engine advances its children and judges its scheduler there. The wait is a bound on a
/// run that never arrives, not a pace the verdict depends on, so a driver passes the state it is
/// waiting for rather than a count of ticks.
func tickUntil<Policy: StatePolicy>(
    _ engine: Engine<Policy>, timeout: Duration = .seconds(3),
    _ reached: () -> Bool
) -> Bool {
    let deadline = ContinuousClock.now + timeout
    while !reached() {
        if ContinuousClock.now >= deadline {
            return false
        }
        engine.tick()
        Thread.sleep(forTimeInterval: 0.005)
    }
    return true
}

/// Starts `engine` for a helper that hands a started machine to several tests, and records a start
/// that was refused (docs/adr/0096) as an issue of the test that asked for it: what the test then
/// reads of a machine that was not started fails it too.
func startRecordingARefusal<Policy: StatePolicy>(_ engine: Engine<Policy>) {
    do {
        try engine.initialize()
    } catch {
        Issue.record("the machine's start was refused: \(error)")
    }
}

/// Starts a machine on a manual clock, so the drive below owns every instant the run sees. Throws
/// what refused the start (docs/adr/0096).
func initializedOnVirtualTime<Policy: StatePolicy>(_ policy: Policy) throws -> Engine<Policy> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    return engine
}

/// Drives a machine started with `initializedOnVirtualTime` until `done` holds (by default, until
/// it is in a final state), moving its clock only to the next moment it says it is owed work, and
/// says whether `done` held when the drive ended.
///
/// The drive ends without it when the machine owes nothing more, or when the next thing it owes
/// lies beyond `budgetMs` of virtual time. Neither depends on how fast the host runs this thread,
/// so the same calls reach the same configuration on an idle machine and on one running every
/// suite at once; `budgetMs` bounds how long the DOCUMENT may take, not how late the host may be.
func driveOnVirtualTime<Policy: StatePolicy>(
    _ engine: Engine<Policy>, budgetMs: Int64 = 2000, maxSteps: Int = 10_000,
    until done: ((Engine<Policy>) -> Bool)? = nil
) -> Bool {
    let reached = done ?? { $0.isInFinalState }
    var remaining = budgetMs
    var steps = 0
    while !reached(engine) {
        engine.tick()
        if reached(engine) {
            break
        }
        // `nil` is "nothing is owed", by this machine or any child it ticks; the tick above is
        // done, so no further time can change that.
        guard let due = engine.timeUntilNextScheduled() else {
            break
        }
        let (seconds, attoseconds) = due.components
        let dueMs = seconds * 1000 + (attoseconds + 999_999_999_999_999) / 1_000_000_000_000_000
        if dueMs > remaining {
            break
        }
        engine.advanceTimeMs(dueMs)
        remaining -= dueMs
        steps += 1
        if steps > maxSteps {
            Issue.record(
                "the machine asked to be ticked \(maxSteps) times without reaching what the test waits for or consuming its \(budgetMs)ms budget; something re-arms itself at the current instant"
            )
            break
        }
    }
    return reached(engine)
}
