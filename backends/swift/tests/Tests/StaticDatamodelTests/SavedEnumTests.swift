// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// An enum variable is saved as the name its enum document declares for the variant, not as the number it
// is held as (docs/adr/0045): a build that renumbered the variants still reads the state. `static_enum`
// holds two of them, `layout` and `previous`. The shared fixture is the text every backend writes after the
// same run and restores from.

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

private func started() throws -> (StaticEnumPolicy, Engine<StaticEnumPolicy>) {
    let policy = StaticEnumPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    return (policy, engine)
}

private func restored(from text: String) throws -> (StaticEnumPolicy, Engine<StaticEnumPolicy>) {
    let policy = StaticEnumPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(text))
    return (policy, engine)
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

/// The run `static_enum.scxml` describes: month to week and then to the agenda, two changes.
private func run(_ engine: Engine<StaticEnumPolicy>) {
    for event in ["swap", "agenda"] {
        engine.raiseExternalByName(event)
        engine.step()
    }
}

@Test func anEnumIsSavedByTheNameItsDocumentDeclares() throws {
    let (_, engine) = try started()
    run(engine)
    #expect(try engine.save().toJSON() == sharedFixture("static_enum"))
}

@Test func aRestoredMachineHoldsTheVariantsTheSavedOneDid() throws {
    let (policy, engine) = try restored(from: sharedFixture("static_enum"))
    #expect(policy.layout.declaredName == "agenda_list")
    #expect(policy.changes == 2)
    // `back` goes to the variant the screen came from, which only `previous` still holds.
    engine.raiseExternalByName("back")
    engine.step()
    #expect(policy.layout.declaredName == "week")
    #expect(policy.changes == 3)
}

@Test func aNameTheEnumDoesNotDeclareIsRefusedAndNothingIsWritten() throws {
    let text = try sharedFixture("static_enum").replacingOccurrences(of: "\"previous\":\"week\"", with: "\"previous\":\"wide\"")
    let policy = StaticEnumPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    #expect(
        refusal { try engine.restore(SavedState.fromJSON(text)) } == "'previous' (wide) is not a variant of ViewMode")
    #expect(!engine.isRunning)
    #expect(policy.changes == 0, "the variable read before it was not written either")
}

@Test func aNumberWhereAnEnumIsExpectedIsRefused() throws {
    let text = try sharedFixture("static_enum").replacingOccurrences(of: "\"layout\":\"agenda_list\"", with: "\"layout\":3")
    let engine = Engine(policy: StaticEnumPolicy())
    engine.setClock(ManualClock())
    #expect(refusal { try engine.restore(SavedState.fromJSON(text)) } == "'layout' is not a text")
}
