// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045, section 8): the tree fixture saved byte
// for byte, restored from that text, and refused for a grandchild of another shape and a child of a later revision.
//
// Fixture: `static_invoke_tree.scxml`. The shared text, `saved/static_invoke_tree_working.json`, is the tree
// `working` leaves:
//
// - `worker` was started twice (`rest`, `back`), and has taken `go`: it counted a step, holds a delayed `report` to
//   its parent, asks its host for the time (a host-run invocation that has not answered), and runs a grandchild
//   under the hybrid candidate `static_hybrid_watcher`, whose `pick` was changed to the other candidate after it
//   started (`swap`); it keeps the id the grandchild was started under in `helper_id` (`idlocation`).
// - The parent holds a delayed `nudge` to the child (`arm`) and the child's `answer` in its queue; the child holds
//   the parent's `swap` in its own.

private let root = "fixture-root"

/// The wall-clock moment the tree is saved at.
private let savedAtMs: Int64 = 1_700_000_000_000

/// The standard host type the child asks.
private let clock = "sce-host-clock-now"

private func fixture(_ path: String) throws -> String {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then StaticDatamodelTests, Tests, tests, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let file = url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved/\(path).json")
    return try String(contentsOf: file, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
}

/// How many times the host was asked for the time.
private final class Asked {
    var count = 0
}

/// The run the shared text is the save of. The host serves the clock and never answers, so the invocation is running
/// when the tree is saved.
private func working() throws -> (Asked, Engine<StaticInvokeTreePolicy>) {
    let asked = Asked()
    let policy = StaticInvokeTreePolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.registerInvoker(clock) { event in
        if case .start = event { asked.count += 1 }
        return nil
    }
    try engine.initialize()
    // Started twice: left and entered again.
    for event in ["rest", "back"] {
        engine.raiseExternalByName(event)
        engine.tick()
    }
    // `go`, forwarded in one tick and taken by the child in the next.
    engine.raiseExternalByName("go")
    engine.tick()
    engine.tick()
    #expect(asked.count == 1, "the child asked its host once")
    // `swap`, forwarded and taken: `pick` names the other candidate now.
    engine.raiseExternalByName("swap")
    engine.tick()
    engine.tick()
    // A delayed send to the child.
    engine.raiseExternalByName("arm")
    engine.tick()
    // `ask`, then `swap`, taken two to a call: the child answers `question` at once, behind the host's `swap`, and
    // the call ends after the parent takes `swap`, which it forwards to the child, so the child's answer waits in
    // the parent's queue and the parent's `swap` in the child's.
    try engine.setMaxExternalEventsPerCall(2)
    engine.raiseExternalByName("ask")
    engine.raiseExternalByName("swap")
    engine.tick()
    return (asked, engine)
}

@Test func theTreeIsSavedAsTheTextEveryBackendSaves() throws {
    let (_, engine) = try working()
    #expect(try engine.save(wallNowMs: savedAtMs).toJSON() == fixture("static_invoke_tree_working"))
}

@Test func aTreeIsRestoredFromTheTextEveryBackendSaves() throws {
    let text = try fixture("static_invoke_tree_working")
    let policy = StaticInvokeTreePolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.registerInvoker(clock) { _ in nil }
    try engine.restore(SavedState.fromJSON(text), wallNowMs: savedAtMs)
    #expect(try engine.save(wallNowMs: savedAtMs).toJSON() == text, "what was restored is saved as it was read")
}
