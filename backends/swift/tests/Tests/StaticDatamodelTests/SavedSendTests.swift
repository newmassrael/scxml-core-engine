// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A delayed `<send>` still waiting is part of a saved state (docs/adr/0045): a saved machine that left it out
// would restore into one that never delivers it, and nothing would say so. Each is saved as the wall-clock
// moment it comes due and not as a wait, so a timer that ran out while the process was dead is due as soon as
// the machine is back. The shared fixtures are the text every backend writes after the same run.
//
// `static_timers` arms four sends on entering `waiting`: `inner` (1 s, the internal queue), `beat` and `echo`
// (2 s, the external queue, due together) and `timeout` (5 s, id `timer`, which `stop` cancels). Run to the end
// its `trace` is 1243; 1423 would be `echo` ahead of `beat`.

private let root = "fixture-root"

/// The wall clock the fixtures were written against: the moment the machine started.
private let startedAt: Int64 = 1_700_000_000_000

private func sharedFixture(_ name: String) throws -> String {
    var url = URL(fileURLWithPath: #filePath)
    // The file, then StaticDatamodelTests, Tests, tests, swift and backends.
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    let path = url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved/\(name).json")
    return try String(contentsOf: path, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
}

private func startedTimers() throws -> (StaticTimersPolicy, Engine<StaticTimersPolicy>, ManualClock) {
    let policy = StaticTimersPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    let clock = ManualClock()
    engine.setClock(clock)
    try engine.initialize()
    return (policy, engine, clock)
}

private func restoredTimers(
    from text: String, wallNowMs: Int64
) throws -> (StaticTimersPolicy, Engine<StaticTimersPolicy>) {
    let policy = StaticTimersPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.restore(SavedState.fromJSON(text), wallNowMs: wallNowMs)
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

@Test func theFourWaitingSendsAreTheTextEveryBackendWrites() throws {
    let (_, engine, _) = try startedTimers()
    #expect(try engine.save(wallNowMs: startedAt).toJSON() == sharedFixture("static_timers"))
}

@Test func aSendThatCameDueIsNotSavedAndTheOnesStillWaitingAre() throws {
    // One second in: `inner` was delivered, and `beat`, `echo` and `timeout` still wait for their moments.
    let (_, engine, _) = try startedTimers()
    engine.advanceTimeMs(1000)
    #expect(try engine.save(wallNowMs: startedAt + 1000).toJSON() == sharedFixture("static_timers_midway"))
}

@Test func aSaveReadsTheTimeLeftNotTheTimeTheWaitStartedAt() throws {
    // The same machine, saved at another wall moment, is the same wait ending at another wall moment.
    let (_, engine, _) = try startedTimers()
    let later = try engine.save(wallNowMs: startedAt + 600)
    #expect(later.pending.map(\.due) == [startedAt + 1600, startedAt + 2600, startedAt + 2600, startedAt + 5600])
}

@Test func aRestoredMachineDeliversTheSendsInTheOrderTheSavedOneWould() throws {
    let (saved, savedEngine, _) = try startedTimers()
    let (restored, restoredEngine) = try restoredTimers(from: sharedFixture("static_timers"), wallNowMs: startedAt)
    for _ in 0..<5 {
        savedEngine.advanceTimeMs(1000)
        restoredEngine.advanceTimeMs(1000)
        #expect(restored.trace == saved.trace)
    }
    #expect(restored.trace == 1243, "inner, then beat ahead of echo, then timeout")
    #expect(restoredEngine.isInFinalState && savedEngine.isInFinalState)
}

@Test func aMachineRestoredFromMidwayDeliversWhatWasLeft() throws {
    let (restored, engine) = try restoredTimers(
        from: sharedFixture("static_timers_midway"), wallNowMs: startedAt + 1000)
    #expect(restored.trace == 1)
    engine.advanceTimeMs(1000)
    #expect(restored.trace == 124, "beat and echo are due together, and come in the order they were sent")
    engine.advanceTimeMs(3000)
    #expect(restored.trace == 1243 && engine.isInFinalState)
}

@Test func aTimerThatRanOutWhileTheProcessWasDeadIsDueAsSoonAsTheMachineIsBack() throws {
    // Ten seconds on the wall clock: all four were due while nothing ran, and one drive delivers them in
    // the order their moments gave.
    let (restored, engine) = try restoredTimers(from: sharedFixture("static_timers"), wallNowMs: startedAt + 10_000)
    #expect(restored.trace == 0, "restoring delivers nothing")
    engine.tick()
    #expect(restored.trace == 1243 && engine.isInFinalState)
}

@Test func aSendCancelledAfterTheRestoreIsCancelledByTheIdItWasSavedUnder() throws {
    let (restored, engine) = try restoredTimers(
        from: sharedFixture("static_timers_midway"), wallNowMs: startedAt + 1000)
    engine.raiseExternalByName("stop")
    engine.step()
    engine.advanceTimeMs(10_000)
    #expect(restored.trace == 124, "`timeout` was cancelled by its id, `timer`")
    #expect(!engine.isInFinalState)
}

@Test func aSaveAndARestoreOfTheSameInstantCarryOnAlike() throws {
    let (_, engine, _) = try startedTimers()
    engine.advanceTimeMs(1500)
    let text = try engine.save(wallNowMs: startedAt + 1500).toJSON()
    let (restored, restoredEngine) = try restoredTimers(from: text, wallNowMs: startedAt + 1500)
    #expect(try restoredEngine.save(wallNowMs: startedAt + 1500).toJSON() == text, "the text is a fixed point")
    engine.advanceTimeMs(500)
    restoredEngine.advanceTimeMs(500)
    #expect(restored.trace == 124)
}

@Test func theIdsAMachineGeneratedForASendCarryOnAfterTheRestore() throws {
    // `idlocation` ids come from a count the saved state holds, so the sends a restored machine arms are
    // never named as one the earlier run already handed to the document.
    let policy = StaticSendIdlocationPolicy()
    try policy.setSessionID(root)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    engine.raiseExternalByName("arm")
    engine.step()
    let text = try engine.save(wallNowMs: startedAt).toJSON()
    #expect(text == (try sharedFixture("static_send_idlocation_armed")))

    let restored = StaticSendIdlocationPolicy()
    let restoredEngine = Engine(policy: restored)
    restoredEngine.setClock(ManualClock())
    try restoredEngine.restore(SavedState.fromJSON(text), wallNowMs: startedAt)
    restoredEngine.raiseExternalByName("cancel.first")
    restoredEngine.step()
    restoredEngine.advanceTimeMs(200)
    #expect(restored.firstFired == 0 && restored.secondFired == 1, "the id `first` held is the one that was cancelled")
    restoredEngine.raiseExternalByName("arm")
    restoredEngine.step()
    #expect(try restoredEngine.save(wallNowMs: startedAt + 200).pending.map { send -> String in
        guard case .raise(_, _, let id, _) = send.act else { return "" }
        return id
    } == ["_auto_send_3", "_auto_send_4"])
}

@Test func aSendNamingAnEventTheDocumentLacksIsRefusedAndLeavesTheMachineAsItWas() throws {
    let (_, engine, _) = try startedTimers()
    var saved = try engine.save(wallNowMs: startedAt)
    saved.pending[1].act = .raise(event: "nowhere", data: "", sendID: "__send_0", sender: root)
    let policy = StaticTimersPolicy()
    let restoredEngine = Engine(policy: policy)
    restoredEngine.setClock(ManualClock())
    #expect(
        refusal { try restoredEngine.restore(saved, wallNowMs: startedAt) }
            == "pending[1] is 'nowhere', which the document does not name")
    #expect(!restoredEngine.isRunning, "none of the sends was armed and the machine did not start")
}

@Test func aSendWithNoIdIsRefused() throws {
    let (_, engine, _) = try startedTimers()
    var saved = try engine.save(wallNowMs: startedAt)
    saved.pending[0].act = .internal(event: "inner", data: "", sendID: "")
    let restoredEngine = Engine(policy: StaticTimersPolicy())
    restoredEngine.setClock(ManualClock())
    let said = refusal { try restoredEngine.restore(saved, wallNowMs: startedAt) }
    #expect(said?.hasPrefix("pending[0] has no send id") == true, "\(said ?? "none")")
}

@Test func aSendWaitingForAChildThisDocumentDoesNotInvokeIsRefusedByName() throws {
    let (_, engine, _) = try startedTimers()
    var saved = try engine.save(wallNowMs: startedAt)
    saved.pending[0].act = .child(
        invokeID: "ghost", event: "inner", data: "", sendID: "__send_2", sender: root)
    let restoredEngine = Engine(policy: StaticTimersPolicy())
    restoredEngine.setClock(ManualClock())
    #expect(
        refusal { try restoredEngine.restore(saved, wallNowMs: startedAt) }
            == "pending[0] is a send to 'ghost', which the document does not invoke")
}

@Test func aSendWaitingForTheParentIsSavedAndRestoredAsTheSessionThatStartedThisOne() throws {
    // A root has no parent, but the route is a session the tree holds, so the text carries it as it does any
    // send: delivered when due, it finds no one to take it, and the sender is told (error.communication).
    let (_, engine, _) = try startedTimers()
    engine.scheduleRoutedEvent(
        nil, delay: .seconds(3), sendID: "up", eventData: "", sender: root,
        route: ScheduledRoute(kind: .parent, eventName: "up"))
    let saved = try SavedState.fromJSON(engine.save(wallNowMs: startedAt).toJSON())
    #expect(
        saved.pending.first { $0.due == startedAt + 3000 }?.act
            == .parent(event: "up", data: "", sendID: "up", sender: root))
    let restoredEngine = Engine(policy: StaticTimersPolicy())
    restoredEngine.setClock(ManualClock())
    try restoredEngine.restore(saved, wallNowMs: startedAt)
    #expect(try restoredEngine.save(wallNowMs: startedAt).toJSON() == saved.toJSON(), "the text is a fixed point")
}

@Test func aWaitingSendTheFormatCannotCarryRefusesTheSaveByName() throws {
    let (_, engine, _) = try startedTimers()
    engine.scheduleHTTPSend(
        delay: .seconds(3), target: "http://example.test/hook", eventName: "posted", params: [:], sendID: "post",
        sender: root)
    #expect(
        refusal { _ = try engine.save(wallNowMs: startedAt) }
            == "the delayed send 'post' is an HTTP POST, which a saved state does not carry")

    let (_, other, _) = try startedTimers()
    other.scheduleRoutedEvent(
        nil, delay: .seconds(3), sendID: "kid", eventData: "", sender: root,
        route: ScheduledRoute(kind: .session, eventName: "up", sessionID: "elsewhere"))
    #expect(
        refusal { _ = try other.save(wallNowMs: startedAt) }
            == "the delayed send 'kid' is routed to a child session, which a saved state does not carry")
}

@Test func aHostServedSendWaitsAsTheRequestItMadeAndIsPerformedAfterTheRestore() throws {
    let (_, engine, _) = try startedTimers()
    let request = HostSendRequest(
        processorType: "x:host", eventName: "ping", target: "dock", content: "hello", params: ["k": ["a", "b"]],
        sendID: "hs", eventData: "{\"k\":[\"a\",\"b\"]}", invokeID: "")
    engine.scheduleHostSend(delay: .seconds(4), request: request, sender: root)
    let saved = try SavedState.fromJSON(engine.save(wallNowMs: startedAt).toJSON())
    let waiting = saved.pending.first { $0.due == startedAt + 4000 }
    #expect(
        waiting?.act
            == .host(
                processorType: "x:host", event: "ping", target: "dock", content: "hello", params: ["k": ["a", "b"]],
                sendID: "hs", data: "{\"k\":[\"a\",\"b\"]}", invokeID: ""))

    let restoredEngine = Engine(policy: StaticTimersPolicy())
    restoredEngine.setClock(ManualClock())
    let performed = Performed()
    try restoredEngine.registerEventProcessor("x:host") { performing in
        performed.requests.append(performing)
        return []
    }
    try restoredEngine.restore(saved, wallNowMs: startedAt)
    restoredEngine.advanceTimeMs(3999)
    #expect(performed.requests.isEmpty)
    restoredEngine.advanceTimeMs(1)
    #expect(performed.requests == [request])
}

private final class Performed {
    var requests: [HostSendRequest] = []
}
