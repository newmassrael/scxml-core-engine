// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A state saved in format 2 is restored by this build and saved again as format 4 (docs/adr/0090, section 7): each
// origin is read by the table of that section, and an `internal` act's stored sender is not read. The fixtures in
// `saved/format2/` are restored, each with a machine of its document, at the wall-clock moment the shared fixtures
// use, and saved at once; the save is, byte for byte, the text of the same name in `saved/format2/read_as/`.
//
// - `static_invoke_tree_working` is the shared tree as the backends other than C11 wrote it, and
//   `static_invoke_tree_working_c11` the same tree with the queued `answer` stored as C11 stored it, the sender's
//   location; both are read as the shared tree.
// - `static_invoke_tree_working_origins` holds in the root's queue one event for each row of the table.
// - `static_timers` is the shared timers, whose `internal` act stored the sender.
//
// A state saved in format 3 is restored and saved again as format 4 too (docs/adr/0107, D4): the child's running
// host-run invocation is the first start of its `<invoke>`, and the save is the shared text of the same name.

/// The wall-clock moment the fixtures were saved at.
private let savedAtMs: Int64 = 1_700_000_000_000

/// `saved/`, or the directory `under` names within it.
private func savedDirectory(_ under: String = "") -> URL {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then StaticDatamodelTests, Tests, tests, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let directory = url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved")
    return under.isEmpty ? directory : directory.appendingPathComponent(under)
}

/// `saved/format2/`, or the directory `under` names within it.
private func format2Directory(_ under: String = "") -> URL {
    savedDirectory(under.isEmpty ? "format2" : "format2/\(under)")
}

private func fixtureText(_ directory: URL, _ name: String) throws -> String {
    try String(contentsOf: directory.appendingPathComponent("\(name).json"), encoding: .utf8)
        .trimmingCharacters(in: .whitespacesAndNewlines)
}

/// `name` in `directory` restored into a tree whose child asks its host for the time and is never answered, and
/// saved at once.
private func treeSavedAgain(_ name: String, from directory: URL = format2Directory()) throws -> String {
    let engine = Engine(policy: StaticInvokeTreePolicy())
    engine.setClock(ManualClock())
    try engine.registerInvoker("sce-host-clock-now") { _ in nil }
    try engine.restore(SavedState.fromJSON(fixtureText(directory, name)), wallNowMs: savedAtMs)
    return try engine.save(wallNowMs: savedAtMs).toJSON()
}

@Test func everyFormat2TreeIsSavedAgainAsTheFormat4TextItIsReadAs() throws {
    let names = ["static_invoke_tree_working", "static_invoke_tree_working_c11", "static_invoke_tree_working_origins"]
    for name in names {
        #expect(try treeSavedAgain(name) == fixtureText(format2Directory("read_as"), name), "\(name)")
    }
}

@Test func theFormat3TreeIsSavedAgainAsTheSharedFormat4Tree() throws {
    let name = "static_invoke_tree_working"
    #expect(try treeSavedAgain(name, from: savedDirectory("format3")) == fixtureText(savedDirectory(), name))
    let written = try FileManager.default.contentsOfDirectory(atPath: savedDirectory("format3").path)
        .filter { $0.hasSuffix(".json") }
    #expect(written == ["\(name).json"], "a fixture added to `saved/format3/` would be read by no Swift test")
}

@Test func theFormat2TimersAreSavedAgainAsTheFormat4TextTheyAreReadAs() throws {
    let engine = Engine(policy: StaticTimersPolicy())
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(fixtureText(format2Directory(), "static_timers")), wallNowMs: savedAtMs)
    #expect(
        try engine.save(wallNowMs: savedAtMs).toJSON() == fixtureText(format2Directory("read_as"), "static_timers"))
}

@Test func everyFormat2ReadingFixtureIsAskedOfThisBuild() throws {
    // A fixture added to `saved/format2/` and not to the two tests above would be read by no Swift test.
    let written = try FileManager.default.contentsOfDirectory(atPath: format2Directory().path)
        .filter { $0.hasSuffix(".json") }.sorted()
    #expect(
        written == [
            "static_invoke_tree_working.json", "static_invoke_tree_working_c11.json",
            "static_invoke_tree_working_origins.json", "static_timers.json",
        ])
}
