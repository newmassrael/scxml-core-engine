// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A `datamodel="sce-static"` machine saved at a macrostep boundary and restored into a new process (SCE
// Accepted Subset, "Saving and restoring"; docs/adr/0045): where it stood, every variable, the events still on
// its external queue and the session id, written as the document every backend reads and made again from it.
//
// `static_counter` holds three numbers and a truth value and nothing else: no history, no child, no delayed
// send, so what is saved here is the variables, where the machine stands and its queue. Each state goes
// through its JSON text and back before it is restored, as one that crossed a process boundary would.

private let sessionID = "fixture-root"

/// A counter that has started on a clock the test holds.
private func started() throws -> (StaticCounterPolicy, Engine<StaticCounterPolicy>) {
    let policy = StaticCounterPolicy()
    try policy.setSessionID(sessionID)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    return (policy, engine)
}

/// A counter that has not started, for a restore to stand where a saved one was.
private func unstarted() throws -> (StaticCounterPolicy, Engine<StaticCounterPolicy>) {
    let policy = StaticCounterPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    return (policy, engine)
}

private func tick(_ engine: Engine<StaticCounterPolicy>, _ times: Int) {
    for _ in 0..<times {
        engine.raiseExternalByName("tick")
        engine.step()
    }
}

private func throughJSON(_ saved: SavedState) throws -> SavedState {
    try SavedState.fromJSON(saved.toJSON())
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

@Test func aRestoredMachineCarriesOnWhereTheSavedOneStood() throws {
    // Seven ticks: count 7, and `ready` set at 5 and not yet cleared. `step` is the machine's own, saved
    // though no snapshot publishes it.
    let (saved, savedEngine) = try started()
    tick(savedEngine, 7)
    let (restored, restoredEngine) = try unstarted()
    try restoredEngine.restore(throughJSON(savedEngine.save()))
    #expect(restored.count == saved.count)
    #expect(restored.ready == saved.ready)
    #expect(restored.count == 7 && restored.ready)

    tick(savedEngine, 2)
    tick(restoredEngine, 2)
    #expect(restored.count == saved.count, "both run on alike: count 9")
    #expect(restored.ready == saved.ready, "and the <elseif> cleared ready")
    #expect(restored.count == 9 && !restored.ready)
}

@Test func aSavedStateIsTheTextEveryBackendWrites() throws {
    let (_, engine) = try started()
    tick(engine, 7)
    let shape = StaticCounterPolicy.savedShape.format2First
    #expect(
        try engine.save().toJSON()
            == """
            {"format":3,"shape":"\(shape)","sessionid":"fixture-root","configuration":["counting"],\
            "current":"counting","variables":{"count":7,"ready":true,"step":1},"history":{},"pending":[],\
            "invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}
            """)
}

@Test func theSessionIDASavedStateCarriesIsTheOneARestoreBinds() throws {
    let (_, engine) = try started()
    let (restored, restoredEngine) = try unstarted()
    #expect(restored.sessionID != sessionID, "a machine that has not started drew an id of its own")
    try restoredEngine.restore(engine.save())
    #expect(restored.sessionID == sessionID, "the session carries on, so the id the tree names it by does too")
}

@Test func theEventsStillOnTheExternalQueueAreSavedAndDeliveredAfterTheRestore() throws {
    // Raised and not yet driven through: the internal queue is empty at a macrostep boundary, the external
    // one is not, and a state that left it out would lose the event.
    let (_, engine) = try started()
    engine.raiseExternalByName("tick")
    let saved = try throughJSON(engine.save())
    #expect(saved.external.map(\.name) == ["tick"])
    #expect(saved.external.first?.type == "external")
    let (restored, restoredEngine) = try unstarted()
    try restoredEngine.restore(saved)
    #expect(restored.count == 0, "restoring delivered nothing")
    restoredEngine.step()
    #expect(restored.count == 1, "the event that was waiting arrives once the machine is driven")
}

@Test func aStateSavedFromAnotherDocumentIsRefused() throws {
    let (_, engine) = try started()
    var saved = try engine.save()
    saved.shape = String(repeating: "0", count: 64)
    let (_, restoredEngine) = try unstarted()
    let said = refusal { try restoredEngine.restore(saved) }
    #expect(said?.contains("shape") == true, "\(said ?? "none")")
    #expect(!restoredEngine.isRunning)
}

@Test func aRefusedRestoreLeavesTheMachineAsItWasAndWritesNothing() throws {
    let (_, engine) = try started()
    tick(engine, 5)
    let text = try engine.save().toJSON()
    // `count` reads before `ready` fails, and still nothing is written: the judgments come first.
    let broken = text.replacingOccurrences(of: #""ready":true"#, with: #""ready":"yes""#)
    let (restored, restoredEngine) = try unstarted()
    let said = refusal { try restoredEngine.restore(SavedState.fromJSON(broken)) }
    #expect(said == "'ready' is not a truth value")
    #expect(!restoredEngine.isRunning, "the machine did not start")
    #expect(restored.count == 0, "and none of the variables the state held was written")
    // The same machine restores the intact state afterwards.
    try restoredEngine.restore(SavedState.fromJSON(text))
    #expect(restored.count == 5)
}

@Test func aVariableTheTypeCannotHoldAndAMissingOneAreRefusedByName() throws {
    let (_, engine) = try started()
    let text = try engine.save().toJSON()
    let wide = text.replacingOccurrences(of: #""count":0"#, with: #""count":4294967296"#)
    let (_, first) = try unstarted()
    #expect(
        refusal { try first.restore(SavedState.fromJSON(wide)) }
            == "'count' (4294967296) is not a whole number its type can hold")
    let missing = text.replacingOccurrences(of: #","step":1"#, with: "")
    let (_, second) = try unstarted()
    #expect(refusal { try second.restore(SavedState.fromJSON(missing)) } == "the saved state has no variable 'step'")
}

@Test func aConfigurationThatIsNotOneOfTheDocumentIsRefused() throws {
    let (_, engine) = try started()
    let text = try engine.save().toJSON().replacingOccurrences(of: #""configuration":["counting"]"#, with: #""configuration":["nowhere"]"#)
    let (_, restoredEngine) = try unstarted()
    let said = refusal { try restoredEngine.restore(SavedState.fromJSON(text)) }
    #expect(said == "the saved configuration names 'nowhere', which is no state of this document")
}

@Test func aMachineThatIsNotRunningIsNotSavedAndAStartedOneIsNotRestored() throws {
    let (_, fresh) = try unstarted()
    #expect(refusal { _ = try fresh.save() } == "the machine is not running, so there is nothing to save: start it first")
    let (_, engine) = try started()
    let saved = try engine.save()
    #expect(
        refusal { try engine.restore(saved) }
            == "the machine has started: a saved state is restored into one that has not")
}

@Test func aMachineThatHasEndedAtATopLevelFinalIsNotSaved() throws {
    let (_, engine) = try started()
    tick(engine, 5)
    engine.raiseExternalByName("go")
    engine.step()
    #expect(engine.isInFinalState)
    #expect(
        refusal { _ = try engine.save() }
            == "the machine has ended at a top-level <final>, so there is nothing to resume")
}

@Test func aStateThatNamesAChildOrAHistoryIsNotRestoredByAMachineThatHasNone() throws {
    let (_, engine) = try started()
    let saved = try engine.save()

    var withChild = saved
    withChild.invokes = .resumed([
        SavedChild(id: "kid", start: 1, invokeID: "counting.1.kid", document: "kid", session: saved)
    ])
    #expect(
        refusal { try unstarted().1.restore(withChild) }
            == "the saved state holds a running child session 'kid', which this machine does not start")

    var withHistory = saved
    withHistory.history = [SavedEntry("h", ["counting"])]
    #expect(
        refusal { try unstarted().1.restore(withHistory) }
            == "the saved state records the history 'h', which the document does not declare")
}

@Test func aQueuedEventKeepsItsSenderBesideTheReplyAddressItsProcessorWrote() throws {
    // docs/adr/0090, section 1: the sender is the session that sent the event and the origin the reply address
    // the delivering processor wrote, two fields no rule ties together. A remote child's event the Mesh transport
    // delivers names the child beside its endpoint, and the machine that holds it is saved and restored with both.
    let (_, engine) = try started()
    let endpoint = EventOrigin(processor: scxmlEventProcessorType, address: "mesh://dev/counter/root.work.1.kid")
    engine.raiseExternalByNameWithMeta(
        "tick", metadata: EventMetadata(eventType: .externalEvent, origin: endpoint, sender: "root.work.1.kid"))
    let text = try engine.save().toJSON()
    #expect(
        text.contains(
            #""origin":{"processor":"\#(scxmlEventProcessorType)","address":"mesh://dev/counter/root.work.1.kid"},"sender":"root.work.1.kid""#
        ), "\(text)")
    let saved = try SavedState.fromJSON(text)
    #expect(saved.external.first?.origin == endpoint)
    #expect(saved.external.first?.sender == "root.work.1.kid")
    let (restored, restoredEngine) = try unstarted()
    try restoredEngine.restore(saved)
    #expect(try restoredEngine.save().toJSON() == text, "the pair and the sender are restored as they were saved")
    restoredEngine.step()
    #expect(restored.count == 1, "the event is delivered after the restore")
}
