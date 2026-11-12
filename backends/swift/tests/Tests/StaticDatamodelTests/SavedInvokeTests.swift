// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A child session an `<invoke>` started is part of what a machine is doing, so a saved machine that left it out
// would restore into a state that says "working" with nobody working (docs/adr/0045). `static_invoke` invokes
// `worker`, a child that takes two events in order, `a` then `b`, and ends; the parent forwards what it is sent.
// A format-2 state holds the child with its whole state and a restore resumes it where it stood; a format-1 state
// names it by its id alone and a restore starts it again from its beginning.

private let root = "fixture-root"

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

private func started() throws -> (StaticInvokePolicy, Engine<StaticInvokePolicy>) {
    let policy = StaticInvokePolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    return (policy, engine)
}

/// `text` restored into a machine given the root's id, so a format-1 text, which holds no id, saves again as the
/// shared format-2 text does.
private func restored(_ text: String) throws -> (StaticInvokePolicy, Engine<StaticInvokePolicy>) {
    let policy = StaticInvokePolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(text))
    return (policy, engine)
}

/// Sends each event and runs rounds of `tick` after it, as the scenarios do: a child session is run by its parent's
/// `tick`, and an event takes a round to be forwarded, one for the child's turn and one for what it sent back.
private func send(_ engine: Engine<StaticInvokePolicy>, _ names: String...) {
    for name in names {
        engine.raiseExternalByName(name)
        for _ in 0..<5 {
            engine.tick()
        }
    }
}

@Test func aMachineWithARunningChildSavesTheTextEveryBackendSaves() throws {
    let (_, engine) = try started()
    #expect(try engine.save().toJSON() == fixture("static_invoke_working"))
}

@Test func aChildSavedAfterItsFirstEventIsResumedWhereItStood() throws {
    let (policy, engine) = try started()
    send(engine, "a")
    let text = try engine.save().toJSON()
    #expect(text.contains(#""configuration":["second"]"#), "the child is saved where it stands, past `a`")
    let (restoredPolicy, restoredEngine) = try restored(text)
    #expect(restoredPolicy.completed == 0)
    // A child resumed in `second` ends on `b` alone.
    send(restoredEngine, "b")
    #expect(restoredPolicy.completed == 1, "the resumed child took `b` and ended: done.invoke.worker arrived")
    send(engine, "b")
    #expect(policy.completed == 1, "the machine it was saved from ends the same way")
}

@Test func aSavedStateIsAFixedPointOfRestoreAndSave() throws {
    let text = try fixture("static_invoke_working")
    let (_, engine) = try restored(text)
    #expect(try engine.save().toJSON() == text)
}

@Test func aStateOfTheFirstFormatStartsTheChildAgainFromItsBeginning() throws {
    let (policy, engine) = try restored(fixture("format1/static_invoke_working"))
    // The restart is the child's first start under this machine's counting, so what it saves is the second
    // format's text of the same moment.
    #expect(try engine.save().toJSON() == fixture("static_invoke_working"))
    // The child began again: `b` alone does not end it, and `a` then `b` does.
    send(engine, "b")
    #expect(policy.completed == 0)
    send(engine, "a", "b")
    #expect(policy.completed == 1)
}

@Test func aMachineWhoseChildHasEndedSavesNoChild() throws {
    let (_, engine) = try started()
    send(engine, "a", "b")
    let saved = try engine.save()
    #expect(saved.invokes.ids.isEmpty, "an ended child is not running, though the state that invokes it stays")
    #expect(saved.invokeStarts == [SavedEntry("worker", 1)])
}

@Test func aChildTheDocumentDoesNotStartIsRefusedByName() throws {
    let text = try fixture("static_invoke_working").replacingOccurrences(of: #""id":"worker","start""#, with: #""id":"ghost","start""#)
    let said = refusal { _ = try restored(text) }
    #expect(
        said == "the saved state holds a running child session 'ghost', which this machine does not start",
        "\(said ?? "none")")
}

@Test func aChildOfAnotherDocumentOrAnotherCountOrAnotherIDIsRefusedAndNothingIsStarted() throws {
    let whole = try fixture("static_invoke_working")
    let other = whole.replacingOccurrences(
        of: #""document":"static_invoke__sce_synth_invoke__worker""#, with: #""document":"elsewhere""#)
    #expect(
        refusal { _ = try restored(other) }
            == "invokes[0] is of 'elsewhere', which 'worker' does not start")
    let counted = whole.replacingOccurrences(of: #""start":"1","invokeid""#, with: #""start":"2","invokeid""#)
    let counting = refusal { _ = try restored(counted) }
    #expect(counting?.hasPrefix("invokes[0] is start 2 of 'worker', and the session counts 1 starts of it") == true, "\(counting ?? "none")")
    let named = whole.replacingOccurrences(of: #""invokeid":"working.1.worker""#, with: #""invokeid":"working.7.worker""#)
    let naming = refusal { _ = try restored(named) }
    #expect(
        naming
            == "invokes[0].session's id is 'fixture-root.working.1.worker', and a child of 'fixture-root' started under 'working.7.worker' is 'fixture-root.working.7.worker'",
        "\(naming ?? "none")")
}

@Test func aChildWhoseOwnStateIsNotOneOfItsDocumentIsRefusedByItsPath() throws {
    let text = try fixture("static_invoke_working").replacingOccurrences(
        of: #""configuration":["first"],"current":"first""#, with: #""configuration":["nowhere"],"current":"nowhere""#)
    let said = refusal { _ = try restored(text) }
    #expect(
        said == "the session at invokes[0].session: the saved configuration names 'nowhere', which is no state of this document",
        "\(said ?? "none")")
    // A refused restore started nothing: the machine is left as it was, and restores the intact state afterwards.
    let engine = Engine(policy: StaticInvokePolicy())
    engine.setClock(ManualClock())
    #expect(refusal { try engine.restore(SavedState.fromJSON(text)) } != nil)
    #expect(!engine.isRunning)
    try engine.restore(SavedState.fromJSON(fixture("static_invoke_working")))
    #expect(engine.isRunning)
}

@Test func aMachineWhoseStateDoesNotHoldTheInvokeCannotHoldItsChild() throws {
    let text = try fixture("static_invoke_working").replacingOccurrences(
        of: #""configuration":["working"],"current":"working""#, with: #""configuration":["idle"],"current":"idle""#)
    let said = refusal { _ = try restored(text) }
    #expect(said == "invokes[0] is 'worker', whose state the saved configuration does not stand in", "\(said ?? "none")")
}
