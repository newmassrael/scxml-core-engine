// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md). Swift AOT path, the flat lifecycle.
//
// The document declares `sce:transition-function="emit"`, so its machine is generated with a pure
// function of (state, event) beside it, in the caseless enum `FlatLifecycleTransitionFunction`. The
// function was computed at build time by the selection rule the engine follows at run time; this
// drives the machine to every state by the function's shortest path, sends every event there, and
// holds what the engine does to what the function says, pair by pair.
//
// Fixture: integration_resources/flat_lifecycle/flat_lifecycle.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

private typealias Function = FlatLifecycleTransitionFunction

/// The machine of the pair being checked: a probe starts a fresh one for each.
private final class Run {
    var policy = FlatLifecyclePolicy()
    var engine = Engine(policy: FlatLifecyclePolicy())
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct FlatLifecycle {
    @Test func theMachineAndItsTransitionFunctionAgreeOnEveryPair() throws {
        let run = Run()
        let probe = EngineProbe(
            start: {
                run.policy = FlatLifecyclePolicy()
                run.engine = Engine(policy: run.policy)
                try run.engine.initialize()
            },
            send: { name in
                if let event = run.policy.getEventFromName(name) {
                    run.engine.processEvent(event)
                }
            },
            active: { Set(run.engine.getActiveStates().map { run.policy.getStateName($0) }) },
            terminal: { run.engine.terminalState.map { run.policy.getStateName($0) } },
            discarded: { Int(run.engine.discardedExternalEvents) })
        let function = TransitionFunctionView<Function.State, Function.Event>(
            states: Function.allStates,
            events: Function.allEvents,
            initial: Function.initial,
            transition: Function.transition,
            isFinal: Function.isFinal,
            configuration: Function.configuration,
            stateName: { $0.scxmlName },
            eventName: { $0.scxmlName })
        #expect(
            try expectEveryPairAgrees(function, probe) == Function.allStates.count * Function.allEvents.count,
            "the drive did not check every pair")
    }
}
