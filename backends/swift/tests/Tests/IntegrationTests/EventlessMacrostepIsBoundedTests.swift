// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: a macrostep whose eventless chain cannot end is stopped, and the host is told.
// Swift AOT path.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The ceiling the engine applies, spelled here rather than read back from it. A test that asked
/// the engine for its own limit would agree with any limit, including one an edit moved by three
/// orders of magnitude.
private let maxMicrosteps: Int64 = 1000

/// One lap of either chain is two microsteps (`_a` to `_b`, then back) and only the `_a` edge
/// counts, so a chain run to the ceiling records half.
private let lapsAtCeiling: Int64 = maxMicrosteps / 2

/// The fixture counts chain laps with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: EventlessMacrostepIsBoundedPolicy, engine: Engine<EventlessMacrostepIsBoundedPolicy>
) {
    let policy = EventlessMacrostepIsBoundedPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EventlessMacrostepIsBounded {
    /// The axis: the macrostep is cut at a known microstep, and the count says it was.
    @Test func aMacrostepThatCannotEndIsStopped() {
        let (policy, engine) = started()
        #expect(
            engine.truncatedMacrosteps == 0,
            "nothing has been refused before the machine has done anything")

        engine.processEvent(.spin)

        #expect(
            policy.readScriptValue("spins") == .integer(lapsAtCeiling),
            "the chain must run exactly as far as the engine allows: fewer means the document was cut off early, more means the ceiling moved"
        )
        #expect(
            engine.truncatedMacrosteps == 1,
            "the microstep past the budget was enabled and was not taken; without this count the host cannot learn that the configuration it reads is not a stable one"
        )
        #expect(
            engine.lastTruncatedMacrostepState == .spinA,
            "an eventless cycle is a closed walk through the state graph; this names a state on it"
        )
    }

    /// The other half, and the one that makes the count mean something: a chain that ends on its
    /// own is not refused, however long it is. The fixture's bounded chain is exactly
    /// `maxMicrosteps` microsteps for this reason: a ceiling that counted loop turns rather than
    /// microsteps taken, or that tested `>=` where it meant `>`, reports this ordinary document as
    /// a runaway.
    @Test func aChainThatEndsAtTheCeilingIsNotRefused() {
        let (policy, engine) = started()

        engine.processEvent(.bounded)

        #expect(
            policy.readScriptValue("laps") == .integer(lapsAtCeiling),
            "the guard `laps < 500` closes after five hundred laps, so the chain is a thousand microsteps long and then stops by itself"
        )
        #expect(
            engine.truncatedMacrosteps == 0,
            "nothing was refused: the macrostep reached the stable configuration, using every microstep it was allowed"
        )
        #expect(
            engine.lastTruncatedMacrostepState == nil,
            "and nothing names a state, because nothing was stopped")
        #expect(engine.getCurrentState() == .boundedA, "the chain rests where its guard closed")
    }

    /// A count, not a flag: a second unbounded macrostep is refused the same way the first was.
    @Test func aSecondTruncatedMacrostepCountsAgain() {
        let (policy, engine) = started()

        engine.processEvent(.spin)
        #expect(engine.truncatedMacrosteps == 1, "precondition: this test is about the SECOND refusal")

        // `reset` is the fixture's way back out of the cycle, and it moves the machine on purpose.
        engine.processEvent(.reset)
        #expect(engine.getCurrentState() == .idle, "reset is the way back out of the chain")

        engine.processEvent(.spin)

        #expect(
            engine.truncatedMacrosteps == 2,
            "the second macrostep hit the same ceiling and must be counted again: a count that saturated at one would read as a machine that recovered"
        )
        #expect(
            policy.readScriptValue("spins") == .integer(2 * lapsAtCeiling),
            "and it really bought the document a full budget again rather than refusing on sight"
        )
    }

    /// The control: an ordinary document is untouched by any of this. Without it, an engine that
    /// refused every macrostep would pass the assertions above and fail nothing.
    @Test func anOrdinaryMacrostepIsNotCounted() {
        let (policy, engine) = started()

        engine.processEvent(.poke)

        #expect(policy.readScriptValue("pokes") == .integer(1), "the run fired")
        #expect(
            engine.truncatedMacrosteps == 0,
            "a macrostep of one microstep ends the way the clause says it does")
        #expect(engine.lastTruncatedMacrostepState == nil)
        #expect(engine.getCurrentState() == .idle)
    }
}
