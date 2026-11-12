// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// What each `<history>` recorded when its parent was last exited is part of a saved state (docs/adr/0045), so
// a restored machine resumes through a history as the saved one would have. `static_history` declares three: a
// shallow one, a deep one and a deep one below a `<parallel>`. The shared fixtures are the text every backend
// writes after the same run and restores from.

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

/// The run `static_history.scxml` describes: `last`, `deepest` and `crew_all` all recorded, standing in `paused`.
private let historyRun = ["faster", "boost", "pause", "work", "left_next", "right_next", "break"]

private func run(_ engine: Engine<StaticHistoryPolicy>, _ events: [String]) {
    for event in events {
        engine.raiseExternalByName(event)
        engine.step()
    }
}

private func started() throws -> (StaticHistoryPolicy, Engine<StaticHistoryPolicy>) {
    let policy = StaticHistoryPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    return (policy, engine)
}

private func restored(from text: String) throws -> (StaticHistoryPolicy, Engine<StaticHistoryPolicy>) {
    let policy = StaticHistoryPolicy()
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

@Test func aHistoryIsSavedWithTheMachineAsTheTextEveryBackendSaves() throws {
    let (_, engine) = try started()
    run(engine, historyRun)
    #expect(try engine.save().toJSON() == sharedFixture("static_history"))
}

@Test func aHistorySavedInFormat1IsRestored() throws {
    let (_, engine) = try restored(from: sharedFixture("format1/static_history"))
    run(engine, ["resume_deep"])
    #expect(engine.getCurrentState() == .burst)
}

@Test func aMachineThatHasExitedNothingRecordsNoHistory() throws {
    let (_, engine) = try started()
    #expect(try engine.save().history.isEmpty)
}

@Test func aRestoredMachineResumesThroughEachHistoryAsTheSavedOneWould() throws {
    // Shallow: the child `fast` was active, and entering it takes its initial child.
    let (shallow, shallowEngine) = try restored(from: sharedFixture("static_history"))
    run(shallowEngine, ["resume_last"])
    #expect(shallowEngine.getCurrentState() == .cruise)
    #expect(shallow.resumed == 1)

    // Deep: the atomic state itself.
    let (_, deepEngine) = try restored(from: sharedFixture("static_history"))
    run(deepEngine, ["resume_deep"])
    #expect(deepEngine.getCurrentState() == .burst)

    // Deep, below a `<parallel>`: one state in each region.
    let (_, crewEngine) = try restored(from: sharedFixture("static_history"))
    run(crewEngine, ["resume_crew"])
    let configuration = crewEngine.getActiveStates()
    for state in [StaticHistoryState.l2, .r2] {
        #expect(configuration.contains(state), "\(configuration)")
    }
    for state in [StaticHistoryState.l1, .r1] {
        #expect(!configuration.contains(state), "\(configuration)")
    }
}

@Test func aRestoredMachineSavesWhatItRestoredAndTheRunThatFollowsRecordsOnTopOfIt() throws {
    let text = try sharedFixture("static_history")
    let (_, engine) = try restored(from: text)
    #expect(try engine.save().toJSON() == text, "a restore then a save is the text it was")
}

@Test func aHistoryThatTheDocumentDoesNotDeclareOrCouldNotHaveRecordedIsRefused() throws {
    let text = try sharedFixture("static_history")
    #expect(
        refusal { _ = try restored(from: text.replacingOccurrences(of: #""last":"#, with: #""ghost":"#)) }
            == "the saved state records the history 'ghost', which the document does not declare")
    #expect(
        refusal { _ = try restored(from: text.replacingOccurrences(of: #""crew_all":["l2","r2"]"#, with: #""crew_all":["l2","nowhere"]"#)) }
            == "the history 'crew_all' records 'nowhere', which the document does not name")
    // A state outside the history's parent is no state the history could have recorded.
    let outside = refusal { _ = try restored(from: text.replacingOccurrences(of: #""last":["fast"]"#, with: #""last":["l1"]"#)) }
    #expect(outside?.hasPrefix("the history 'last' records states it could not have: ") == true, "\(outside ?? "none")")
    #expect(
        refusal { _ = try restored(from: text.replacingOccurrences(of: #""deepest":["burst"]"#, with: #""deepest":["burst","burst"]"#)) }?
            .contains("records burst twice") == true)
}
