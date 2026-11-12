// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// SCE Accepted Subset §2.15, "Saving and restoring": an `<invoke>` the HOST runs is part of what a machine is doing
// (§scxml-6.4.1), and the process that ran it is gone with the process that saved it. A saved state holds each
// running host-run invocation with the request it was started with and the deadline it had left, and a restore
// starts it again from that request, never evaluating the element a second time, with `restarted` set, a new token
// and what was left of the deadline.
//
// `static_host_http` hands a request to the host's `sce-host-http-fetch` invoker from `fetching`, with a deadline of
// five seconds of the engine's clock. Driven on `ManualClock` throughout: no case sleeps.

private let root = "fixture-root"

/// The wall-clock moment the machine is saved at: 2023-11-14T22:13:20Z.
private let savedAtMs: Int64 = 1_700_000_000_000

private let fetchType = "sce-host-http-fetch"

/// What the host was told, in order.
private final class Host {
    var told: [HostInvokeEvent] = []

    func handle(_ event: HostInvokeEvent) -> HostInvokeResponse? {
        told.append(event)
        return nil
    }

    var starts: [HostInvokeRequest] {
        told.compactMap { event in
            if case .start(let request) = event { return request }
            return nil
        }
    }

    var cancels: [HostInvokeCancel] {
        told.compactMap { event in
            if case .cancel(let cancel) = event { return cancel }
            return nil
        }
    }
}

private func machine(_ host: Host, sessionID: String? = nil) throws -> (StaticHostHttpPolicy, Engine<StaticHostHttpPolicy>) {
    let policy = StaticHostHttpPolicy()
    if let sessionID {
        try policy.setSessionID(sessionID)
    }
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.registerInvoker(fetchType) { host.handle($0) }
    return (policy, engine)
}

/// A machine that has asked its host to fetch, and whose host has not answered.
private func fetching() throws -> (Host, StaticHostHttpPolicy, Engine<StaticHostHttpPolicy>) {
    let host = Host()
    let (policy, engine) = try machine(host, sessionID: root)
    try engine.initialize()
    engine.raiseHostHttpFetchRequest(method: "GET", url: "https://example.test/a", headers: "accept: text/plain", body: "")
    engine.step()
    return (host, policy, engine)
}

private func restored(
    _ saved: SavedState, wallNowMs: Int64 = savedAtMs
) throws -> (Host, StaticHostHttpPolicy, Engine<StaticHostHttpPolicy>) {
    let host = Host()
    let (policy, engine) = try machine(host)
    try engine.restore(SavedState.fromJSON(saved.toJSON()), wallNowMs: wallNowMs)
    return (host, policy, engine)
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

@Test func aMachineIsSavedWithTheRequestItsInvocationWasStartedWith() throws {
    let (host, _, engine) = try fetching()
    #expect(host.starts.count == 1, "the host was asked once")
    let saved = try engine.save(wallNowMs: savedAtMs)
    #expect(saved.hostInvokes.count == 1)
    let running = try #require(saved.hostInvokes.first)
    #expect(running.processorType == fetchType)
    #expect(running.invokeID == "fetch")
    #expect(running.params["url"] == ["https://example.test/a"])
    #expect(running.params["_sce_deadline_ms"] == nil, "the deadline is the engine's, not the host's")
    #expect(running.due == savedAtMs + 5000, "the time left, written as the wall-clock moment it comes due")
    #expect(saved.hostInvokeToken == 1, "one start has been made")
}

@Test func aRestoreAsksNothingOfTheHostUntilTheMachineIsDriven() throws {
    let (_, _, engine) = try fetching()
    let (host, _, restoredEngine) = try restored(engine.save(wallNowMs: savedAtMs))
    #expect(host.told.isEmpty, "the host registers its invokers after restoring, so nothing was asked")
    #expect(restoredEngine.timeUntilNextScheduled() == .zero, "a restored invocation owes the machine a tick now")
}

@Test func aRestoredInvocationIsStartedAgainFromTheRequestItWasSavedWith() throws {
    let (firstHost, _, engine) = try fetching()
    let first = try #require(firstHost.starts.first)
    let (host, _, restoredEngine) = try restored(engine.save(wallNowMs: savedAtMs))
    restoredEngine.step()
    let again = try #require(host.starts.first)
    #expect(host.starts.count == 1)
    #expect(again.restarted, "the host is told this start is a restore starting it again")
    #expect(again.processorType == first.processorType && again.invokeID == first.invokeID)
    #expect(again.params == first.params && again.src == first.src && again.content == first.content)
    #expect(again.eventData == first.eventData, "the data is what the document's own start made, not made again")
    #expect(again.token != first.token, "a new start: what the first run hands back is not the second run's")
}

@Test func aRestoredInvocationKeepsTheDeadlineItHadLeft() throws {
    let (_, _, engine) = try fetching()
    // Two seconds of the five were used before the machine was saved.
    engine.advanceTimeMs(2000)
    let saved = try engine.save(wallNowMs: savedAtMs + 2000)
    #expect(try #require(saved.hostInvokes.first).due == savedAtMs + 5000)
    let (host, policy, restoredEngine) = try restored(saved, wallNowMs: savedAtMs + 2000)
    restoredEngine.step()
    #expect(host.starts.count == 1)
    restoredEngine.advanceTimeMs(2999)
    #expect(policy.outcome == "", "three seconds were left, and 2999 have passed")
    restoredEngine.advanceTimeMs(1)
    #expect(policy.outcome == "deadline", "the deadline ended the invocation at the moment it had left")
    #expect(host.cancels.count == 1, "and the host was told to stop")
}

@Test func aDeadlineThatPassedWhileTheMachineWasAwayEndsTheInvocationUnstarted() throws {
    let (_, _, engine) = try fetching()
    let saved = try engine.save(wallNowMs: savedAtMs)
    let (host, policy, restoredEngine) = try restored(saved, wallNowMs: savedAtMs + 6000)
    restoredEngine.step()
    #expect(host.starts.isEmpty, "the host is not asked to begin what it would be told to stop at once")
    #expect(policy.outcome == "deadline", "the document receives the error.invoke the deadline raises")
}

@Test func aCompletionOfTheFirstRunIsRefusedAndOneOfTheSecondRunIsAccepted() throws {
    let (firstHost, _, engine) = try fetching()
    let firstToken = try #require(firstHost.starts.first).token
    let (host, _, restoredEngine) = try restored(engine.save(wallNowMs: savedAtMs))
    restoredEngine.step()
    let secondToken = try #require(host.starts.first).token
    #expect(
        !restoredEngine.completeHostInvoke(
            processorType: fetchType, invokeID: "fetch", token: firstToken, doneData: "{}"),
        "what the first run hands back cannot answer for the second")
    #expect(
        restoredEngine.completeHostInvoke(
            processorType: fetchType, invokeID: "fetch", token: secondToken, doneData: "{}"))
}

@Test func aMachineRestoredAndSavedAgainBeforeItWasDrivenKeepsItsInvocation() throws {
    let (_, _, engine) = try fetching()
    let saved = try engine.save(wallNowMs: savedAtMs)
    let (_, _, restoredEngine) = try restored(saved)
    #expect(try restoredEngine.save(wallNowMs: savedAtMs).toJSON() == saved.toJSON())
}

@Test func anInvocationCancelledBeforeItWasStartedAgainIsNeverStarted() throws {
    let (_, _, engine) = try fetching()
    let (host, _, restoredEngine) = try restored(engine.save(wallNowMs: savedAtMs))
    #expect(!restoredEngine.cancelHostInvoke(processorType: fetchType, invokeID: "fetch"), "nothing was started to stop")
    restoredEngine.step()
    #expect(host.starts.isEmpty)
    #expect(host.cancels.isEmpty)
}

@Test func aSavedStateNamingAHostInvocationTheDocumentCannotHaveRunIsRefused() throws {
    let (_, _, engine) = try fetching()
    let saved = try engine.save(wallNowMs: savedAtMs)

    var other = saved
    other.hostInvokes[0].processorType = "x-other"
    #expect(
        refusal { _ = try restored(other) }
            == "hostinvokes[0] is 'fetch' of type 'x-other', which the document does not have a host invoker run")

    var elsewhere = saved
    elsewhere.configuration = ["idle"]
    elsewhere.current = "idle"
    #expect(
        refusal { _ = try restored(elsewhere) }
            == "hostinvokes[0] is 'fetch', whose state the saved configuration does not stand in")

    var twice = saved
    twice.hostInvokes.append(saved.hostInvokes[0])
    #expect(
        refusal { _ = try restored(twice) } == "hostinvokes[1] is 'fetch', which an earlier entry already names")
}
