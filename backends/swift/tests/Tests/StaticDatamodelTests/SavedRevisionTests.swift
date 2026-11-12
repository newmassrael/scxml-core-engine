// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A state saved at an earlier revision of a record schema or an enum restores, each field added since at its
// default (docs/adr/0044, docs/adr/0055). `static_schema_revision` holds a record of the third revision of a schema
// whose second added `layout` (default `week`) and third `urgent` (default `true`); `static_schema_pending` holds
// a record whose schema and whose enum both moved on, with a send still waiting. The fixtures are the text every
// backend writes and restores from.

private let root = "fixture-root"

/// The wall-clock moment the pending fixture was saved at.
private let pendingSavedAtMs: Int64 = 1_700_000_000_000

private func fixture(_ path: String) throws -> String {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then StaticDatamodelTests, Tests, tests, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let file = url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved/\(path).json")
    return try String(contentsOf: file, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
}

private func refusal(_ body: () throws -> Void) -> String? {
    do {
        try body()
        return nil
    } catch let refusal as StateRefusal {
        return refusal.message
    } catch {
        return "\(error)"
    }
}

/// `text` restored into a machine given the root's id, so a format-1 text, which holds no id, saves again as the
/// shared format-2 text does.
private func revised(_ text: String) throws -> (StaticSchemaRevisionPolicy, Engine<StaticSchemaRevisionPolicy>) {
    let policy = StaticSchemaRevisionPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(text))
    return (policy, engine)
}

/// What the machine's `last` holds, as a saved state writes it.
private func lastOf(_ engine: Engine<StaticSchemaRevisionPolicy>) throws -> String {
    writeJSON(try engine.save().variable("last"))
}

@Test func aRecordOfASchemaThatGrewSavesTheTextEveryBackendSaves() throws {
    // A delivery of the first revision: the machine is at the third, so it names that revision and the revised
    // shape.
    let policy = StaticSchemaRevisionPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    engine.raiseExternalByName("reading.revised", eventData: #"{"level":4}"#)
    engine.step()
    #expect(try engine.save().toJSON() == fixture("static_schema_revision"))
}

@Test func aStateSavedAtTheFirstRevisionRestoresWithEveryLaterDefault() throws {
    let (policy, engine) = try revised(fixture("format1/static_schema_revision_first"))
    #expect(try engine.save().toJSON() == fixture("static_schema_revision"))
    #expect(policy.taken == 1)
}

@Test func aStateSavedAtTheSecondRevisionKeepsItsLayoutAndTakesTheUrgency() throws {
    let (policy, engine) = try revised(fixture("format1/static_schema_revision_second"))
    #expect(try lastOf(engine) == #"{"level":5,"layout":"day","urgent":true}"#)
    #expect(policy.taken == 2)
}

@Test func aQueuedDeliveryOfTheFirstRevisionTakesTheDefaultsWhenItIsDelivered() throws {
    // The external queue holds the data as the wire carried it; the lift reads it with the defaults when the
    // event is dequeued.
    let (policy, engine) = try revised(fixture("format1/static_schema_revision_first_queued"))
    #expect(try lastOf(engine) == #"{"level":4,"layout":"week","urgent":true}"#)
    engine.step()
    #expect(try lastOf(engine) == #"{"level":5,"layout":"week","urgent":true}"#)
    #expect(policy.taken == 2)
    #expect(policy.refusals == 0)
}

@Test func aStateOfALaterOrAnotherRevisionIsRefused() throws {
    func refusalOf(_ text: String) -> String {
        refusal { try Engine(policy: StaticSchemaRevisionPolicy()).restore(SavedState.fromJSON(text)) } ?? "none"
    }
    // Saved by a build of a fourth revision this one does not know.
    let later = try fixture("static_schema_revision").replacingOccurrences(of: #""revision":3"#, with: #""revision":4"#)
    #expect(refusalOf(later).contains("later than"), "\(refusalOf(later))")
    // A second revision that declared other fields than this build's second.
    let other = try fixture("format1/static_schema_revision_second").replacingOccurrences(
        of: "d9ba6a71207a3f219929c8fda298d7a35c76cb7950cc287ae2fa8a8b8ff45ad7",
        with: "e511eddacf1d4e220c85f78c95988fe26a2a1319cf119cb085f1a84f4f197f8d")
    #expect(refusalOf(other).contains("declared other members"), "\(refusalOf(other))")
    // A record saved at the first revision that holds a field the second added.
    let early = try fixture("format1/static_schema_revision_first").replacingOccurrences(
        of: #"{"level":4}"#, with: #"{"level":4,"layout":"day"}"#)
    #expect(refusalOf(early).contains("'last' holds 'layout'"), "\(refusalOf(early))")
    // A state that names its revision under the first shape, which a reader that predates revisions would have
    // taken.
    let unmarked = try fixture("static_schema_revision").replacingOccurrences(
        of: "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5",
        with: "11dbc3cc8b1f883cc9da011d2d5c3968ac0eeb404a0f494deede91552cd6623d")
    #expect(refusalOf(unmarked).contains("names revisions"), "\(refusalOf(unmarked))")
}

@Test func aRevisionNamingNoSchemaOfTheMachineIsRefusedByName() throws {
    let text = try fixture("static_schema_revision").replacingOccurrences(of: "reading.revised", with: "nowhere.else")
    #expect(
        refusal { try Engine(policy: StaticSchemaRevisionPolicy()).restore(SavedState.fromJSON(text)) }
            == "'revisions' names 'nowhere.else', which no record or enum of this machine is")
}

// MARK: A waiting send saved at an earlier revision

private func pending(_ text: String) throws -> (StaticSchemaPendingPolicy, Engine<StaticSchemaPendingPolicy>) {
    let policy = StaticSchemaPendingPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(text), wallNowMs: pendingSavedAtMs)
    return (policy, engine)
}

private func pendingLast(_ engine: Engine<StaticSchemaPendingPolicy>, at wallNowMs: Int64) throws -> String {
    writeJSON(try engine.save(wallNowMs: wallNowMs).variable("last"))
}

@Test func aWaitingSendSavedAtTheFirstRevisionIsDeliveredWithTheLaterDefaults() throws {
    let (policy, engine) = try pending(fixture("format1/static_schema_pending_first"))
    // The record saved without the urgency takes its default.
    #expect(try pendingLast(engine, at: pendingSavedAtMs) == #"{"level":"low","count":0,"urgent":true}"#)
    #expect(policy.taken == 0)
    // The send still waits its second, and its data, which lacks the urgency, is lifted with the default when
    // it is delivered.
    engine.advanceTimeMs(999)
    #expect(policy.taken == 0, "not due yet")
    engine.advanceTimeMs(1)
    #expect(policy.taken == 1, "delivered at its moment")
    #expect(policy.refusals == 0)
    #expect(
        try pendingLast(engine, at: pendingSavedAtMs + 1000) == #"{"level":"high","count":7,"urgent":true}"#)
    // Saved again, the state names both revisions it now holds: the schema's by its event, then the enum's by its
    // document, relative to the machine's.
    let saved = try engine.save(wallNowMs: pendingSavedAtMs + 1000)
    #expect(saved.revisions.map { [$0.key, String($0.value.revision)] } == [["signal.raised", "2"], ["enum:enum_signal_level.scxml", "2"]])
    // The revised shape of format 2, which names the enum by its document; the format-1 text this restored from
    // named it by its alias, under another shape.
    #expect(saved.shape == "3fbeae2b4200145eef7af1a2e393f315a5466a7f5ff65b7bcabbb7ed2e75c9fa")
}

@Test func aValueOfAVariantItsSavedRevisionLacksIsRefused() throws {
    // `critical` came with the enum's second revision, so a state saved at the first cannot hold it.
    let text = try fixture("format1/static_schema_pending_first").replacingOccurrences(
        of: #""level":"low""#, with: #""level":"critical""#)
    let said = refusal { _ = try pending(text) }
    #expect(said?.contains("'last.level' holds 'critical'") == true, "\(said ?? "none")")
}
