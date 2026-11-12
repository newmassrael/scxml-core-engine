// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: a discarded event is something the host can see. Swift AOT path.
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The fixture counts handled events with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: DiscardedEventIsObservablePolicy, engine: Engine<DiscardedEventIsObservablePolicy>
) {
    let policy = DiscardedEventIsObservablePolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct DiscardedEventIsObservable {
    @Test func anEventNoActiveStateAnsweredIsCounted() {
        let (_, engine) = started()
        #expect(engine.discardedExternalEvents == 0, "nothing has been discarded before the first event")

        // `settle` is declared in `busy`, so it is in the machine's vocabulary and the host can
        // name it; it just matches nothing in `idle`.
        engine.processEvent(.settle)

        #expect(
            engine.discardedExternalEvents == 1,
            "`settle` came off the external queue in `idle`, where no transition matches it: the clause discards it, and the host that queued it has no other way to learn its event went nowhere"
        )
        #expect(engine.getCurrentState() == .idle, "a discarded event must not move the machine")
    }

    @Test func aHandledEventIsNotCounted() {
        let (policy, engine) = started()

        engine.processEvent(.poke)
        #expect(
            policy.readScriptValue("pokes") == .integer(1),
            "`poke`'s self transition did not run, so nothing below is measuring a handled event")
        #expect(
            engine.discardedExternalEvents == 0,
            "`poke` matched a self transition: handled, and the configuration is unchanged only because the transition returns to its own source"
        )

        engine.processEvent(.nudge)
        #expect(
            policy.readScriptValue("nudges") == .integer(1),
            "`nudge`'s targetless transition did not run")
        #expect(
            engine.discardedExternalEvents == 0,
            "`nudge` matched a targetless internal transition: its actions ran and no state was exited or entered, which is why the count cannot be keyed off whether the configuration changed"
        )
    }

    @Test func theDiscardIsNotDerivableFromAnyOtherAccessor() {
        let (_, engine) = started()

        engine.processEvent(.poke)
        let handledState = engine.getCurrentState()
        let handledFinal = engine.isInFinalState

        engine.processEvent(.settle)

        #expect(
            engine.getCurrentState() == handledState,
            "this fixture exists because a handled event and a discarded one are indistinguishable through the accessors a host had; if they ever differ, the fixture stopped measuring what it claims"
        )
        #expect(engine.isInFinalState == handledFinal)
        #expect(
            engine.discardedExternalEvents == 1,
            "the two are indistinguishable through every other accessor, so the count is the only thing that separates them"
        )
    }

    @Test func theEngineNamesTheEventItDiscarded() {
        let (_, engine) = started()
        #expect(engine.lastDiscardedEvent == nil, "nothing has been discarded yet")

        engine.processEvent(.settle)

        #expect(
            engine.lastDiscardedEvent == .settle,
            "the engine counted a discard but cannot say which event it was")
    }

    @Test func anEventTheMachineHasMovedPastIsCounted() {
        let (_, engine) = started()
        engine.processEvent(.go)
        #expect(
            engine.getCurrentState() == .busy, "`go` should have moved the machine out of `idle`")

        engine.processEvent(.poke)

        #expect(
            engine.discardedExternalEvents == 1,
            "the machine left `idle`, so `poke` no longer matches: the host that kept sending it is exactly who the count is for"
        )
        #expect(engine.lastDiscardedEvent == .poke)
    }
}
