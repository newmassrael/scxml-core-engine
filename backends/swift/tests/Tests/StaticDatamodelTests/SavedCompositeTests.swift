// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A record, a list and a value that may be absent are saved as the text every backend writes (docs/adr/0045,
// docs/adr/0055): a record is an object of its schema's fields keyed by their ids, a list an array, an absent
// value `null`. The shared fixtures are the text every backend writes after the same run and restores from.

private let root = "fixture-root"

private func sharedFixture(_ name: String) throws -> String {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then StaticDatamodelTests, Tests, tests, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let path = url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved/\(name).json")
    return try String(contentsOf: path, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
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

/// A machine started on a clock the test holds. Its session id is the one the shared fixtures give their root,
/// which the caller binds first (`setSessionID` is each generated policy's own).
private func started<P: SavedPolicy>(_ policy: P) -> Engine<P> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    startRecordingARefusal(engine)
    return engine
}

/// A machine that has not started, for a restore to stand where a saved one was.
private func unstarted<P: SavedPolicy>(_ policy: P) -> Engine<P> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    return engine
}

// MARK: A record

@Test func aRecordIsSavedAsTheObjectOfItsFields() throws {
    let policy = StaticRecordPolicy()
    try policy.setSessionID(root)
    let engine = started(policy)
    engine.raiseDayPicked(year: 2027, month: 2, dayOfMonth: 27)
    engine.step()
    #expect(try engine.save().toJSON() == sharedFixture("static_record"))
}

@Test func aRestoredRecordHoldsTheFieldsTheSavedOneDid() throws {
    let policy = StaticRecordPolicy()
    let engine = unstarted(policy)
    try engine.restore(SavedState.fromJSON(sharedFixture("static_record")))
    #expect(policy.shown == StaticRecordDayRecord(year: 2027, month: 2, dayOfMonth: 27))
    #expect(try engine.save().toJSON() == sharedFixture("static_record"), "what was restored is saved as it was read")
}

@Test func aRecordMissingAFieldOrHoldingOneItsTypeCannotHoldIsRefused() throws {
    let text = try sharedFixture("static_record")
    let missing = text.replacingOccurrences(of: #""month":2,"#, with: "")
    #expect(
        refusal { try unstarted(StaticRecordPolicy()).restore(SavedState.fromJSON(missing)) }
            == "'shown' has no field 'month'")
    let wide = text.replacingOccurrences(of: #""month":2"#, with: #""month":300"#)
    #expect(
        refusal { try unstarted(StaticRecordPolicy()).restore(SavedState.fromJSON(wide)) }
            == "'shown.month' (300) is not a whole number its type can hold")
    let text2 = text.replacingOccurrences(of: #""shown":{"year":2027,"month":2,"dayOfMonth":27}"#, with: #""shown":7"#)
    #expect(
        refusal { try unstarted(StaticRecordPolicy()).restore(SavedState.fromJSON(text2)) }
            == "'shown' is not an object")
}

// MARK: A list

private func pick(_ engine: Engine<StaticListPolicy>, _ dayOfMonth: UInt8) {
    engine.raiseDayPicked(year: 2026, month: 9, dayOfMonth: dayOfMonth)
    engine.step()
}

@Test func aListIsSavedAsAnArrayInOrder() throws {
    let policy = StaticListPolicy()
    try policy.setSessionID(root)
    let engine = started(policy)
    pick(engine, 4)
    pick(engine, 2)
    #expect(try engine.save().toJSON() == sharedFixture("static_list"))
}

@Test func aRestoredListHoldsItsElementsAndTakesTheNextAppend() throws {
    let policy = StaticListPolicy()
    let engine = unstarted(policy)
    try engine.restore(SavedState.fromJSON(sharedFixture("static_list")))
    #expect(policy.picked == [4, 2])
    #expect(policy.count == 2)
    pick(engine, 9)
    #expect(policy.picked == [4, 2, 9], "the list carries on where the saved one stood")
}

@Test func aListLongerThanItsBoundOrWithAnElementItsTypeCannotHoldIsRefused() throws {
    let text = try sharedFixture("static_list")
    let long = text.replacingOccurrences(of: #""picked":[4,2]"#, with: #""picked":[1,2,3,4]"#)
    let policy = StaticListPolicy()
    #expect(
        refusal { try unstarted(policy).restore(SavedState.fromJSON(long)) }
            == "'picked' holds 4 elements, past the 3 it is bounded by")
    #expect(policy.picked.isEmpty, "nothing was written")
    let wrong = text.replacingOccurrences(of: #""picked":[4,2]"#, with: #""picked":[4,"x"]"#)
    #expect(
        refusal { try unstarted(StaticListPolicy()).restore(SavedState.fromJSON(wrong)) }
            == "'picked[1]' is not a number")
    let wide = text.replacingOccurrences(of: #""picked":[4,2]"#, with: #""picked":[4,256]"#)
    #expect(
        refusal { try unstarted(StaticListPolicy()).restore(SavedState.fromJSON(wide)) }
            == "'picked[1]' (256) is not a whole number its type can hold")
}

// MARK: A value that may be absent

private func optionalRun() throws -> (StaticOptionalPolicy, Engine<StaticOptionalPolicy>) {
    let policy = StaticOptionalPolicy()
    try policy.setSessionID(root)
    let engine = started(policy)
    engine.raiseDueSet(id: 1, due: 12, title: nil, status: nil)
    engine.step()
    engine.raiseDueSet(id: 2, due: nil, title: "later", status: .inProgress)
    engine.step()
    return (policy, engine)
}

@Test func anAbsentValueIsNullAndAPresentOneIsWrittenAsItsTypeIs() throws {
    let (policy, engine) = try optionalRun()
    #expect(policy.due == nil)
    #expect(try engine.save().toJSON() == sharedFixture("static_optional"))
}

@Test func aRestoredMachineHoldsWhatWasAbsentAbsentAndWhatWasPresentPresent() throws {
    let policy = StaticOptionalPolicy()
    let engine = unstarted(policy)
    try engine.restore(SavedState.fromJSON(sharedFixture("static_optional")))
    #expect(policy.due == nil)
    #expect(
        policy.items == [
            StaticOptionalDueRecord(id: 1, due: 12, title: nil, status: nil),
            StaticOptionalDueRecord(id: 2, due: nil, title: "later", status: .inProgress),
        ])
    #expect(try engine.save().toJSON() == sharedFixture("static_optional"), "what was restored is saved as it was read")
}

@Test func aNullWhereTheTypeIsNotOptionalIsRefusedByName() throws {
    // `id` is a number in every record the schema describes, whatever the fields beside it may be.
    let text = try sharedFixture("static_optional").replacingOccurrences(
        of: #""item":{"id":2,"#, with: #""item":{"id":null,"#)
    #expect(
        refusal { try unstarted(StaticOptionalPolicy()).restore(SavedState.fromJSON(text)) }
            == "'item.id' is absent (null), and its type is not optional")
}

// MARK: A record that may be absent

@Test func anAbsentRecordIsRestoredAbsentAndAPresentOneAsTheObjectOfItsFields() throws {
    let policy = StaticOptionalRecordPolicy()
    let engine = unstarted(policy)
    try engine.restore(SavedState.fromJSON(sharedFixture("static_optional_record")))
    #expect(policy.found == nil)
    #expect(policy.pick == StaticOptionalRecordItemRecord(id: 2, title: "bumped", count: 7))
    #expect(policy.draft == StaticOptionalRecordItemRecord(id: 3, title: "gamma", count: 9))
    #expect(
        policy.items == [
            StaticOptionalRecordItemRecord(id: 1, title: "alpha", count: 2),
            StaticOptionalRecordItemRecord(id: 2, title: "beta", count: 6),
        ])
    #expect(
        try engine.save().toJSON() == sharedFixture("static_optional_record"),
        "what was restored is saved as it was read")
}

@Test func aSavedNullIsRefusedWhereARecordIsNeverAbsent() throws {
    let text = try sharedFixture("static_optional_record").replacingOccurrences(
        of: #""draft":{"id":3,"title":"gamma","count":9}"#, with: #""draft":null"#)
    let policy = StaticOptionalRecordPolicy()
    let said = refusal { try unstarted(policy).restore(SavedState.fromJSON(text)) }
    #expect(said == "'draft' is not an object", "\(said ?? "none")")
}
