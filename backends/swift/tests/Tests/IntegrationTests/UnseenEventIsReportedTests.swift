// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: an event a stopped machine never looked at is something the host can see. Swift
// AOT path.
//
// Fixture: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The fixture counts handled deliveries with `<assign>`, so this is an ECMAScript-datamodel
/// machine.
private func started() -> (
    policy: UnseenEventIsReportedPolicy, engine: Engine<UnseenEventIsReportedPolicy>
) {
    let policy = UnseenEventIsReportedPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct UnseenEventIsReported {
    /// The axis: an event the host queued after the machine stopped is counted.
    @Test func anEventDeliveredAfterTheMachineStoppedIsCounted() {
        let (policy, engine) = started()
        #expect(engine.unseenExternalEvents == 0, "nothing has been refused before the first event")

        engine.processEvent(.poke)
        #expect(
            policy.readScriptValue("pokes") == .integer(1),
            "`poke`'s transition did not run, so nothing below is measuring a machine that was working first"
        )

        engine.processEvent(.finish)
        #expect(
            engine.isInFinalState,
            "`finish` should have taken the machine to its top-level final state")
        #expect(
            engine.unseenExternalEvents == 0,
            "`finish` was itself dequeued and handled: the machine stopped BECAUSE of it, which is not the same as stopping before it"
        )

        engine.processEvent(.poke)

        #expect(
            engine.unseenExternalEvents == 1,
            "the host queued `poke` on a machine that had reached its final state: the loop had already ended, so the event was never dequeued; before this count the host had no way to learn that"
        )
        #expect(
            policy.readScriptValue("pokes") == .integer(1),
            "the refused delivery ran the document's transition anyway: the count would then be reporting something that did not happen"
        )
    }

    /// Why the query has to exist at all: every other accessor answers the same before and after
    /// the refused delivery.
    @Test func theRefusalIsNotDerivableFromAnyOtherAccessor() {
        let (policy, engine) = started()
        engine.processEvent(.finish)

        let beforeState = engine.getCurrentState()
        // Once the run has ended its configuration is empty, so the final it ended in is what a
        // host reads instead, and it must not move either.
        let beforeEnded = engine.terminalState
        let beforeFinal = engine.isInFinalState
        let beforeDiscarded = engine.discardedExternalEvents
        let beforePokes = policy.readScriptValue("pokes")

        engine.processEvent(.poke)

        #expect(
            engine.getCurrentState() == beforeState,
            "this fixture exists because a refused delivery is indistinguishable through the accessors a host had; if they ever differ, the fixture stopped measuring what it claims"
        )
        #expect(engine.terminalState == beforeEnded)
        #expect(engine.isInFinalState == beforeFinal)
        #expect(engine.discardedExternalEvents == beforeDiscarded)
        #expect(policy.readScriptValue("pokes") == beforePokes)

        #expect(
            engine.unseenExternalEvents == 1,
            "the two readings agree on everything else, so this count is the only thing that separates `the machine never looked` from `it looked and nothing matched`"
        )
    }

    /// The distinction the whole axis turns on: a discard and a refusal are different facts, and
    /// each has its own count.
    @Test func aDiscardAndARefusalAreCountedSeparately() {
        let (_, engine) = started()

        engine.processEvent(.poke)
        #expect(engine.discardedExternalEvents == 0, "`poke` matched a targetless transition")
        #expect(engine.unseenExternalEvents == 0, "the machine was running, so nothing was refused")

        engine.processEvent(.finish)
        engine.processEvent(.poke)

        #expect(
            engine.discardedExternalEvents == 0,
            "a refusal must not be reported as a discard: the first says the machine looked and nothing matched, the second says it never looked"
        )
        #expect(engine.unseenExternalEvents == 1)
    }

    /// A count says an event went unlooked-at; a host debugging a supervisor that stopped
    /// answering needs to know which one.
    @Test func theEngineNamesTheEventItNeverLookedAt() {
        let (_, engine) = started()
        #expect(engine.lastUnseenEvent == nil, "nothing has been refused yet")

        engine.processEvent(.finish)
        engine.processEvent(.poke)
        #expect(
            engine.lastUnseenEvent == .poke,
            "the engine counted a refusal but cannot say which event it refused")

        engine.processEvent(.finish)
        #expect(engine.unseenExternalEvents == 2, "the count is a count, not a flag")
        #expect(
            engine.lastUnseenEvent == .finish,
            "the name did not follow the second refusal")
    }
}
