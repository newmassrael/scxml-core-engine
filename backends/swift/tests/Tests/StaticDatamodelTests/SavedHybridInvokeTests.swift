// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>` (§2.13). A format-2 saved state records
// the candidate that was running, with its whole state, and a restore resumes it whatever `srcexpr` reads now: the
// arguments were evaluated when the invocation started (§scxml-6.4), and a resumed child did not start
// (docs/adr/0045). A format-1 state named the invocation alone, and its restore starts it again from its
// beginning, reading `srcexpr` and the arguments from the restored variables as entering the state would: the one
// case in which a child restarts.
//
// `static_invoke_hybrid_saved.scxml` invokes `watch`, whose `srcexpr` is `pick` among `static_hybrid_watcher` (ends
// on the 8 it is handed) and `static_hybrid_holder` (ends on the 7). `base` is 7 when the child starts, and `bump`
// makes it 8 without telling the child; `swap` makes `pick` name the other candidate.

private typealias Hybrid = StaticInvokeHybridSavedPolicy

/// The wall-clock moment the machine is saved at: 2023-11-14T22:13:20Z.
private let savedAtMs: Int64 = 1_700_000_000_000

private func started() -> (Hybrid, Engine<Hybrid>) {
    let policy = Hybrid()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    startRecordingARefusal(engine)
    return (policy, engine)
}

/// Lets the child run and report to its parent: a child session is run by its parent's `tick`.
private func settle(_ engine: Engine<Hybrid>) {
    for _ in 0..<5 {
        engine.tick()
    }
}

private func send(_ engine: Engine<Hybrid>, _ names: String...) {
    for name in names {
        engine.raiseExternalByName(name)
        settle(engine)
    }
}

/// `saved` restored into a machine on a host-owned clock, as a host carries it through its own storage.
private func restoredFrom(_ saved: SavedState) throws -> (Hybrid, Engine<Hybrid>) {
    let policy = Hybrid()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(saved.toJSON()), wallNowMs: savedAtMs)
    return (policy, engine)
}

private func restored(_ engine: Engine<Hybrid>) throws -> (Hybrid, Engine<Hybrid>) {
    try restoredFrom(engine.save(wallNowMs: savedAtMs))
}

/// The same moment as a format-1 state, which names the running invocation by its id alone. The document holds no
/// enum, so its format-1 shape is the format-2 one the save wrote.
private func asFormat1(_ saved: SavedState) -> SavedState {
    var older = saved
    older.invokes = .restarted(saved.invokes.ids)
    older.invokeStarts = []
    older.sessionID = ""
    return older
}

/// The document each running child of `saved` runs.
private func documents(_ saved: SavedState) -> [String] {
    guard case .resumed(let children) = saved.invokes else { return [] }
    return children.map(\.document)
}

private func working(_ policy: Hybrid, _ engine: Engine<Hybrid>) -> Bool {
    policy.getStateName(engine.getCurrentState()) == "working"
}

@Test func aRestoredChildKeepsTheValuesItWasHanded() throws {
    let (_, engine) = started()
    settle(engine)
    // The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and it is not told.
    send(engine, "bump")
    // Resumed, the watcher holds the 7 it was handed: nothing is evaluated again, so it still waits.
    let (policy, restoredEngine) = try restored(engine)
    settle(restoredEngine)
    #expect(policy.completed == 0)
    #expect(working(policy, restoredEngine))
}

@Test func aRestoreResumesTheCandidateThatWasRunningWhateverSrcexprReadsNow() throws {
    let (policy, engine) = started()
    settle(engine)
    // The watcher is the child that is running when the machine is saved, though `pick` now names the holder.
    send(engine, "swap", "bump")
    #expect(policy.completed == 0)
    let saved = try engine.save(wallNowMs: savedAtMs)
    #expect(documents(saved) == ["static_hybrid_watcher"])

    let (restoredPolicy, restoredEngine) = try restoredFrom(saved)
    settle(restoredEngine)
    #expect(
        try documents(restoredEngine.save(wallNowMs: savedAtMs)) == ["static_hybrid_watcher"],
        "the watcher was resumed, not the holder `pick` names now")
    #expect(working(restoredPolicy, restoredEngine), "the watcher waits for an 8 it was not handed")
}

@Test func aChildNothingHasChangedForWaitsAsItDid() throws {
    let (_, engine) = started()
    settle(engine)
    let (policy, restoredEngine) = try restored(engine)
    settle(restoredEngine)
    // `base` is still 7: the watcher waits for an 8, as it did.
    #expect(policy.completed == 0)
    #expect(working(policy, restoredEngine))
}

@Test func aChildThatEndedIsNotStartedAgain() throws {
    let (_, engine) = started()
    settle(engine)
    // In format 1 the watcher started again is handed the 8 `base` holds, and ends.
    send(engine, "bump")
    let (once, onceEngine) = try restoredFrom(asFormat1(engine.save(wallNowMs: savedAtMs)))
    settle(onceEngine)
    #expect(once.completed == 1)

    // Saved once its `done.invoke` was taken, the machine is past `working`, and a restore names no invocation to
    // resume or start.
    #expect(!working(once, onceEngine))
    let (again, againEngine) = try restored(onceEngine)
    settle(againEngine)
    #expect(again.completed == 1, "the child ran twice")
    #expect(!working(again, againEngine))
}

@Test func aFormat1RestoreHandsTheChildTheValuesTheRestoredMachineHolds() throws {
    let (policy, engine) = started()
    settle(engine)
    send(engine, "bump")
    #expect(policy.completed == 0)
    // Started again from its beginning, the watcher is handed 8, and ends.
    let (restarted, restartedEngine) = try restoredFrom(asFormat1(engine.save(wallNowMs: savedAtMs)))
    settle(restartedEngine)
    #expect(restarted.completed == 1)
}

@Test func aFormat1RestoreReadsTheSrcexprAgain() throws {
    let (policy, engine) = started()
    settle(engine)
    send(engine, "swap", "bump")
    #expect(policy.completed == 0)
    // The restored fields name the holder, which is handed 8 and waits for 7. The watcher, which would have ended on
    // that 8, is not what the restore starts.
    let (restarted, restartedEngine) = try restoredFrom(asFormat1(engine.save(wallNowMs: savedAtMs)))
    settle(restartedEngine)
    #expect(restarted.completed == 0, "the watcher was started")
    #expect(working(restarted, restartedEngine), "the holder waits for a 7 it is not handed")
    // The invocation is still running, and a save names the holder.
    #expect(try documents(restartedEngine.save(wallNowMs: savedAtMs)) == ["static_hybrid_holder"])
}
