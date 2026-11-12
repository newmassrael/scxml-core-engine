// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: a macrostep whose `<raise>` chain cannot end is stopped, and the host is told.
// Swift AOT path.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
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

/// One lap of the alternating chain is two microsteps (one internal event, one eventless
/// transition) and only the internal half is counted, so a chain run to the shared ceiling
/// records half.
private let alternatingLapsAtCeiling: Int64 = maxMicrosteps / 2

/// The fixture counts chain links with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: InternalChainIsBoundedPolicy, engine: Engine<InternalChainIsBoundedPolicy>
) {
    let policy = InternalChainIsBoundedPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct InternalChainIsBounded {
    /// The axis: the macrostep is cut at a known microstep, and the count says it was.
    @Test func aRaiseChainThatCannotEndIsStopped() {
        let (policy, engine) = started()
        #expect(
            engine.truncatedMacrosteps == 0,
            "nothing has been refused before the machine has done anything")

        engine.processEvent(.spin)

        #expect(
            policy.readScriptValue("links") == .integer(maxMicrosteps),
            "the chain must run exactly as far as the engine allows: fewer means the document was cut off early, more means the ceiling moved"
        )
        #expect(
            engine.truncatedMacrosteps == 1,
            "the microstep past the budget was queued and was not taken; without this count the host cannot learn that the configuration it reads is not a stable one"
        )
        #expect(
            engine.lastTruncatedMacrostepState == .spin,
            "the count alone says a document somewhere cannot settle; this says where to look"
        )
        #expect(
            engine.getCurrentState() == .spin,
            "the chain was cut, not the machine: refusing to run it forever is the engine's decision to report, not a reason to stop a machine whose other states still work"
        )
    }

    /// The other half, and the one that makes the count mean something: a chain that ends on its
    /// own is not refused, however long it is.
    @Test func aRaiseChainThatEndsAtTheCeilingIsNotRefused() {
        let (policy, engine) = started()

        engine.processEvent(.bounded)

        #expect(
            policy.readScriptValue("laps") == .integer(maxMicrosteps),
            "the guard `laps < 999` stops matching at the thousandth link, which raises nothing, so the queue empties and the chain stops by itself"
        )
        #expect(
            engine.truncatedMacrosteps == 0,
            "nothing was refused: the macrostep reached the stable configuration the clause describes, using every microstep it was allowed"
        )
        #expect(
            engine.lastTruncatedMacrostepState == nil,
            "and nothing names a state, because nothing was stopped")
        #expect(
            engine.getCurrentState() == .bounded,
            "a document that settles on its own rests where its chain ended"
        )
    }

    /// A dequeue that selected nothing is not a microstep, so it spends no budget. A microstep is
    /// taken for a transition that was SELECTED; a dequeue that matched none is the loop turn the
    /// clause does not count. The fixture's `unanswered` chain is `bounded` with one unmatched
    /// event added per link, so the two differ in exactly that and must cost the same.
    @Test func aDequeueThatSelectedNothingSpendsNoBudget() {
        let (policy, engine) = started()

        engine.processEvent(.unanswered)

        #expect(
            policy.readScriptValue("ignores") == .integer(maxMicrosteps),
            "the chain is the same length as `bounded`; the unmatched events between its links are dequeues that selected nothing, and those are not microsteps"
        )
        #expect(
            engine.truncatedMacrosteps == 0,
            "a thousand microsteps and a thousand discards is a thousand microsteps: an engine that counted the discards refuses this document at link five hundred"
        )
        #expect(
            engine.lastTruncatedMacrostepState == nil,
            "and nothing names a state, because nothing was stopped")
        #expect(
            engine.getCurrentState() == .ignoring,
            "the document settled on its own and rests where its chain ended"
        )
    }

    /// The case a per-branch budget lets through: a chain that alternates one `<raise>` with one
    /// eventless transition.
    @Test func anAlternatingChainSpendsOneSharedBudget() {
        let (policy, engine) = started()

        engine.processEvent(.alternate)

        #expect(
            policy.readScriptValue("alts") == .integer(alternatingLapsAtCeiling),
            "the two branches share one budget, so a chain that alternates them gets five hundred laps out of a thousand microsteps"
        )
        #expect(
            engine.truncatedMacrosteps == 1,
            "and the refusal is reported once, whichever branch was holding the budget when it ran out"
        )
        #expect(
            engine.lastTruncatedMacrostepState == .alt,
            "named the same way as any other chain that could not settle"
        )
    }

    /// What the refusal did with the links it would not run: it left them queued. The fixture's
    /// `resume` chain is half again as long as the ceiling, so the first macrostep is refused with
    /// five hundred links still to go and the second one finishes them. What the event driving the
    /// second macrostep does is deliberately not asserted: the counters below are what the
    /// channels share.
    @Test func aRefusedChainIsLeftQueuedForTheNextMacrostep() {
        let (policy, engine) = started()

        engine.processEvent(.resume)
        #expect(
            policy.readScriptValue("beats") == .integer(maxMicrosteps),
            "the first macrostep spends the whole budget on the chain"
        )
        #expect(engine.truncatedMacrosteps == 1)

        engine.processEvent(.poke)

        #expect(
            policy.readScriptValue("beats") == .integer(maxMicrosteps + maxMicrosteps / 2),
            "the second macrostep picked the chain up where the first was cut and ran it to its end: the refused links were left on the queue, not dropped"
        )
        #expect(
            engine.truncatedMacrosteps == 1,
            "and nothing was refused this time: the chain ended on its own inside the budget"
        )
        #expect(engine.getCurrentState() == .resuming, "the chain was cut, not the machine")
    }

    /// The control: an ordinary document is untouched by any of this.
    @Test func anOrdinaryMacrostepIsNotCounted() {
        let (policy, engine) = started()

        engine.processEvent(.poke)

        #expect(
            policy.readScriptValue("pokes") == .integer(1),
            "the run happened: a counter of zero cannot tell an engine that did nothing from one that was never asked"
        )
        #expect(engine.truncatedMacrosteps == 0, "and one transition is not a chain that cannot end")
        #expect(engine.lastTruncatedMacrostepState == nil)
        #expect(engine.getCurrentState() == .idle)
    }
}
