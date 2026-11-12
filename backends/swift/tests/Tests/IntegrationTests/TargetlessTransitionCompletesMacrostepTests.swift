// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: a transition with no target ends a microstep, not the macrostep. Swift AOT path.
//
// Fixture: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The fixture counts what the macrostep reached with `<assign>`, so this is an
/// ECMAScript-datamodel machine.
private func started() -> (
    policy: TargetlessTransitionCompletesMacrostepPolicy,
    engine: Engine<TargetlessTransitionCompletesMacrostepPolicy>
) {
    let policy = TargetlessTransitionCompletesMacrostepPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct TargetlessTransitionCompletesMacrostep {
    /// The axis: a transition that moves nothing still ends a microstep, so the macrostep
    /// continues into whatever its content enabled.
    ///
    /// `chained == 1, polished == 0` is the signature of an engine that resumes the chain only
    /// after a transition that MOVED the machine; `chained == 0` is the signature of one that
    /// never entered the chain at all.
    @Test func aTargetlessTransitionDoesNotEndTheMacrostep() {
        let (policy, engine) = started()

        engine.processEvent(.arm)

        #expect(
            policy.readScriptValue("armed") == .integer(1),
            "the targetless transition ran its content: without this the rest measures a lost event rather than a stopped macrostep"
        )
        #expect(
            policy.readScriptValue("chained") == .integer(1),
            "and the eventless transition that content enabled was taken in the SAME macrostep, which is the whole of what Appendix D's inner loop promises a host"
        )
        #expect(
            policy.readScriptValue("polished") == .integer(1),
            "including the chain's last link, which is targetless itself: an engine that walks the chain only while the machine keeps moving stops exactly here"
        )
        #expect(
            engine.getCurrentState() == .settled,
            "the host must be handed the stable configuration, not the one the machine was passing through"
        )
    }

    /// The other side of the same inner loop: what a targetless transition raises is answered
    /// before the host gets control back.
    @Test func aRaiseFromATargetlessTransitionIsAnsweredInTheSameMacrostep() {
        let (policy, engine) = started()

        engine.processEvent(.ping)

        #expect(
            policy.readScriptValue("answered") == .integer(1),
            "the internal event the targetless transition raised was dequeued and matched inside this macrostep"
        )
        #expect(
            engine.getCurrentState() == .idle,
            "neither transition moves the machine, which is the point: the macrostep has to continue anyway"
        )
    }

    /// The control, and the reason a zero above means anything: a targetless transition that
    /// enables nothing leaves the machine exactly where it was, and having run is still
    /// observable.
    @Test func aTargetlessTransitionThatEnablesNothingChangesNothingElse() {
        let (policy, engine) = started()

        engine.processEvent(.quiet)

        #expect(policy.readScriptValue("quiet") == .integer(1), "the transition fired")
        #expect(
            policy.readScriptValue("chained") == .integer(0),
            "and nothing else did: the eventless transition's guard is still closed, so an engine that walked the chain here would be firing a transition the document did not enable"
        )
        #expect(policy.readScriptValue("polished") == .integer(0))
        #expect(policy.readScriptValue("answered") == .integer(0))
        #expect(engine.getCurrentState() == .idle)
    }

    /// The other microstep that ends where it began: a transition whose target is its own source.
    /// It is not targetless (it has an exit and an entry), but a macrostep loop that continues
    /// only while the configuration keeps changing drops it for the same reason.
    @Test func anEventlessSelfTransitionExitsAndReEnters() {
        let (policy, engine) = started()

        engine.processEvent(.recycle)

        #expect(
            policy.readScriptValue("entries") == .integer(2),
            "the state is entered once by `recycle` and once more by the eventless self transition its entry enabled: a self transition exits and re-enters, so <onentry> runs again"
        )
        #expect(
            engine.getCurrentState() == .recycled,
            "and the guard closes behind it, so the machine rests here rather than spinning"
        )
    }

    /// A macrostep, not a one-shot: the second targetless transition is followed the same way the
    /// first was.
    @Test func theSecondTargetlessTransitionIsFollowedToo() {
        let (policy, engine) = started()

        engine.processEvent(.quiet)
        engine.processEvent(.ping)
        #expect(
            policy.readScriptValue("answered") == .integer(1),
            "precondition: this test is about the SECOND raise")

        engine.processEvent(.ping)

        #expect(
            policy.readScriptValue("answered") == .integer(2),
            "the raise in the third macrostep was answered like the one in the second: the inner loop belongs to every macrostep, not to the first"
        )
        #expect(policy.readScriptValue("quiet") == .integer(1))
        #expect(engine.getCurrentState() == .idle)
    }
}
