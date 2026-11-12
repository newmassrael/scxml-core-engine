// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel refused is something the host can see. Swift AOT path.
//
// Fixture: integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// Content that announces an object and stops: the shape a truncated write, a half-flushed buffer
/// or a serializer that died mid-record actually produces.
private let truncatedObject = #"{"done":"#

/// The same failure announced with `[`, under the other event name, so a channel that reports "the
/// last event" rather than "the last event that lost a payload" cannot pass by accident.
private let truncatedArray = "[1,2"

/// W3C test 562 sends exactly this shape and requires it to arrive as a string. Counting it would
/// make the statistic fire on documents that are working.
private let prose = "just a sentence"

/// What the host meant to send.
private let intactObject = #"{"done":true}"#

/// The fixture guards on `_event.data.done` and counts deliveries with `<assign>`, so this is an
/// ECMAScript-datamodel machine.
private func started() -> (
    policy: UndecodablePayloadIsReportedPolicy, engine: Engine<UndecodablePayloadIsReportedPolicy>
) {
    let policy = UndecodablePayloadIsReportedPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

private func deliver(
    _ engine: Engine<UndecodablePayloadIsReportedPolicy>, _ event: UndecodablePayloadIsReportedEvent,
    _ payload: String
) {
    engine.processEventWithMeta(event, metadata: EventMetadata(data: payload))
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct UndecodablePayloadIsReported {
    /// The axis: content that asked for the structured reading and did not get it is counted.
    @Test func aPayloadThatAnnouncedStructureAndDidNotParseIsCounted() {
        let (policy, engine) = started()
        #expect(engine.undecodablePayloads == 0, "nothing has been delivered before the first event")

        deliver(engine, .answer, truncatedObject)

        #expect(
            policy.readScriptValue("answers") == .integer(1),
            "the `answer` transition did not run, so nothing below is measuring a delivery that reached the document"
        )
        #expect(
            engine.undecodablePayloads == 1,
            "the host sent `\(truncatedObject)`, which announces an object and does not parse as one: the clause correctly delivers it as a string; the host that built it has no other way to learn its payload stopped being structure"
        )
        #expect(
            engine.getCurrentState() == .waiting,
            "the reading a payload got must not change which transition fired")
    }

    /// The other half. A count that also counts success cannot be used to detect failure, and the
    /// reading the clause calls "otherwise" is the NORMAL outcome for a document whose author
    /// wrote prose.
    @Test func proseAndAPayloadThatParsedAreNotCounted() {
        let (policy, engine) = started()

        deliver(engine, .note, prose)
        #expect(policy.readScriptValue("notes") == .integer(1), "the `note` transition did not run")
        #expect(
            engine.undecodablePayloads == 0,
            "`\(prose)` is the third reading working as the clause specifies and as W3C test 562 requires: a diagnostic that fires when nothing is wrong is one nobody reads"
        )

        deliver(engine, .answer, intactObject)
        #expect(
            engine.getCurrentState() == .accepted,
            "the guard `_event.data.done` did not hold for `\(intactObject)`, so the structured reading did not happen and the zero below would be proving nothing"
        )
        #expect(
            engine.undecodablePayloads == 0,
            "a payload that parsed was counted as one that did not")
    }

    /// Why the query has to exist at all: the two deliveries the fixture's comment names are
    /// identical through every accessor a host had.
    @Test func theLossIsNotDerivableFromAnyOtherAccessor() {
        let (brokenPolicy, broken) = started()
        deliver(broken, .answer, truncatedObject)

        let (intactPolicy, intact) = started()
        // Valid JSON, and `done` is genuinely absent: the innocent explanation an operator has to
        // rule out.
        deliver(intact, .answer, #"{"ready":true}"#)

        #expect(
            broken.getCurrentState() == intact.getCurrentState(),
            "this fixture exists because a lost payload and an absent field are indistinguishable through the accessors a host had; if they ever differ, the fixture stopped measuring what it claims"
        )
        #expect(broken.isInFinalState == intact.isInFinalState)
        #expect(
            brokenPolicy.readScriptValue("answers") == intactPolicy.readScriptValue("answers"),
            "the document's own count is the same for both, which is the whole problem")
        #expect(
            broken.undecodablePayloads == 1,
            "the two runs agree on everything else, so this count is the only thing that separates a broken sender from a working one"
        )
        #expect(intact.undecodablePayloads == 0)
    }

    /// A count says a payload was lost; a host debugging a stalled supervisor needs to know which
    /// delivery lost it.
    @Test func theEngineNamesTheDeliveryThatLostItsPayload() {
        let (_, engine) = started()
        #expect(engine.lastUndecodablePayload == nil, "nothing has been delivered yet")

        deliver(engine, .answer, truncatedObject)
        #expect(
            engine.lastUndecodablePayload == .answer,
            "the engine counted a lost payload but cannot say which delivery lost it")

        // A second loss, under the other event name: the accessor has to track the last event THAT
        // LOST A PAYLOAD, not the last event.
        deliver(engine, .note, truncatedArray)
        #expect(engine.undecodablePayloads == 2, "the count is a count, not a flag")
        #expect(engine.lastUndecodablePayload == .note)

        // And a delivery that succeeds must leave both alone: otherwise the last name would drift
        // to whatever arrived most recently.
        deliver(engine, .answer, intactObject)
        #expect(
            engine.getCurrentState() == .accepted,
            "the intact payload did not take the guarded transition, so the two assertions below are not measuring a successful delivery"
        )
        #expect(
            engine.undecodablePayloads == 2,
            "a delivery that parsed moved a count that belongs to ones that did not")
        #expect(
            engine.lastUndecodablePayload == .note,
            "a delivery that parsed moved a name that belongs to one that did not")
    }
}
