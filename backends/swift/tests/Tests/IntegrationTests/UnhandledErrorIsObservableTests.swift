// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: an error nobody answered is something the host can see. Swift AOT path.
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The fixture counts what ran with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: UnhandledErrorIsObservablePolicy, engine: Engine<UnhandledErrorIsObservablePolicy>
) {
    let policy = UnhandledErrorIsObservablePolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct UnhandledErrorIsObservable {
    /// The axis: an error the engine raised that no active state answers.
    @Test func anErrorNoTransitionAnsweredIsCounted() {
        let (policy, engine) = started()
        #expect(
            engine.unhandledErrorEvents == 0, "no error has gone unhandled before the first event")

        engine.processEvent(.boom)

        #expect(
            policy.readScriptValue("booms") == .integer(1),
            "`boom`'s transition did not run, so nothing below is measuring an error raised inside a transition that fired"
        )
        #expect(
            engine.unhandledErrorEvents == 1,
            "`boom`'s second <assign> has an invalid empty location, so the engine raised error.execution, and `idle` declares no transition for it: the host driving this machine has no other way to learn its <assign> failed"
        )
        #expect(
            engine.getCurrentState() == .idle,
            "the error must not move the machine on its own")
    }

    /// The other half: an error the DOCUMENT answered must not be counted.
    @Test func anErrorTheDocumentHandledIsNotCounted() {
        let (policy, engine) = started()

        engine.processEvent(.go)
        #expect(
            engine.getCurrentState() == .guarded,
            "`go` should have moved the machine to the state that answers errors")

        engine.processEvent(.boom)

        #expect(
            policy.readScriptValue("caught") == .integer(1),
            "`guarded`'s error.execution transition did not run, so this test is not measuring a HANDLED error"
        )
        #expect(
            engine.unhandledErrorEvents == 0,
            "the same <assign> failed in `guarded`, where the document does declare a transition for error.execution: the document dealt with it, and counting it would report the author's own error handling as a silent failure"
        )
        #expect(
            engine.lastUnhandledError == nil, "nothing went unhandled, so there is no last one to name"
        )
    }

    /// The boundary: an author's own unmatched `<raise>` is not an error.
    @Test func anAuthorsUnmatchedRaiseIsNotAnUnhandledError() {
        let (policy, engine) = started()

        engine.processEvent(.whisper)

        #expect(
            engine.unhandledErrorEvents == 0,
            "`whisper` raises `unheard` and `retry.error.execution`, neither of which any state answers. Both are discarded exactly as an unmatched error is, and neither is one: the author wrote the raises and the absent handlers. `retry.error.execution` is the sharper half: it CONTAINS `error.` without starting with it, and the clause reserves the prefix, not the substring"
        )
        #expect(
            policy.readScriptValue("heards") == .integer(1),
            "`whisper`'s third raise, `heard`, does match, and the transition it matches did not run: the count above is a byproduct of the internal drain, never its job"
        )
        #expect(
            engine.discardedExternalEvents == 0,
            "`whisper` itself was handled, so the external-queue count stays put: the internal events it raised are not on that queue at all"
        )
    }

    /// Every pre-existing accessor answers the same for both runs.
    @Test func theUnhandledErrorIsNotDerivableFromAnyOtherAccessor() {
        let (_, engine) = started()

        engine.processEvent(.poke)
        let clean = [
            String(describing: engine.getCurrentState()),
            String(describing: engine.isInFinalState),
            String(describing: engine.discardedExternalEvents),
            String(describing: engine.lastDiscardedEvent),
        ]

        engine.processEvent(.boom)
        let failed = [
            String(describing: engine.getCurrentState()),
            String(describing: engine.isInFinalState),
            String(describing: engine.discardedExternalEvents),
            String(describing: engine.lastDiscardedEvent),
        ]

        #expect(
            clean == failed,
            "this fixture exists because these two are indistinguishable through every accessor a host had, including the discard count, which never sees the internal queue: if they ever differ, the fixture stopped measuring what it claims"
        )
        #expect(
            engine.unhandledErrorEvents == 1,
            "the two are indistinguishable through every other accessor, so this count is the only thing that separates a silent failure from a clean run"
        )
    }

    /// A count says something failed; a repair needs the class of error.
    @Test func theEngineNamesTheErrorItDropped() {
        let (_, engine) = started()
        #expect(engine.lastUnhandledError == nil, "nothing has gone unhandled yet")

        engine.processEvent(.boom)

        #expect(
            engine.lastUnhandledError == .errorExecution,
            "`error.execution` is the document's own executable content failing; `error.communication` would be a <send> that could not reach its target: two different repairs, and a bare count separates neither"
        )
    }

    /// The supervisor's failure mode: every round fails identically.
    @Test func aMachineFailingEveryRoundIsCountedEveryRound() {
        let (policy, engine) = started()

        for round in 1...3 {
            engine.processEvent(.boom)
            #expect(
                engine.unhandledErrorEvents == UInt32(round),
                "round \(round) did not add to the count; a supervisor polling this number is exactly who learns the loop is not making progress"
            )
            #expect(
                engine.getCurrentState() == .idle,
                "the machine looks identical on every round, which is the problem")
        }
        #expect(
            policy.readScriptValue("booms") == .integer(3),
            "all three rounds should have run their transition")
    }
}
