// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

import SceMeshZenohKit

@testable import SceMesh

// The conformance scenarios of SceMeshZenohKit (SCE_MESH.md section 20.4), held twice.
//
// First against the in-memory router, which is the executable statement of what a router does: a router that
// fails the scenarios written from the same section would be a wrong statement. Then against clients that
// each break one promise of the binding's seam, each of which a scenario must name. A kit that cannot fail
// proves nothing about the clients that pass it, so this half is the one that says the first half means
// something.

@Test func theInMemoryRouterMeetsTwoMachines() throws {
    try ZenohConformance.twoMachinesMeet(InMemoryZenohHarness())
}

@Test func theInMemoryRouterTellsAMachineThatWentAwayAsLost() throws {
    try ZenohConformance.aMachineThatGoesAwayIsToldAsLost(InMemoryZenohHarness())
}

@Test func theInMemoryRouterCatchesAnAwayDeviceUpFromAHubThatStayedUp() throws {
    try ZenohConformance.aDeviceThatWasAwayCatchesUpFromAHubThatStayedUp(InMemoryZenohHarness())
}

@Test func theInMemoryRouterResyncsAMachineThatLostItsSession() throws {
    try ZenohConformance.aMachineThatLosesItsSessionComesBackAndResyncs(InMemoryZenohHarness())
}

@Test func everyScenarioRunsOnOneHarnessAfterAnother() throws {
    try ZenohConformance.all(InMemoryZenohHarness())
}

// MARK: Clients that break one promise

/// One promise of the seam a client can break.
private enum Flaw: Sendable {
    /// A put carries one byte less than it was given (section 20.2: each put carries exactly the envelope).
    case truncatesPayload

    /// A presence subscription ignores that it was asked for history, so a machine that opens after another
    /// never learns it is up (section 20.3).
    case dropsPresenceHistory

    /// The client never says its session ended, so the core is never told who was lost (section 20.3).
    case neverTellsTheSessionEnded

    /// Closing the client and undeclaring what it holds leaves the presence token where it was, so a machine
    /// that went away is never reported (section 20.3).
    case leavesItsTokenBehind
}

/// A hold that stands for something the client never undeclares.
private final class StuckHold: ZenohHold {
    func undeclare() {}
}

/// An in-memory client that keeps every promise but one.
private final class FlawedClient: ZenohClient {
    private let inner: any ZenohClient
    private let flaw: Flaw

    init(_ inner: any ZenohClient, _ flaw: Flaw) {
        self.inner = inner
        self.flaw = flaw
    }

    func subscribe(
        filter: String, onSample: @escaping @Sendable (String, [UInt8]) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        try inner.subscribe(filter: filter, onSample: onSample)
    }

    func subscribePresence(
        filter: String, history: Bool, onChange: @escaping @Sendable (String, ZenohPresence) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        try inner.subscribePresence(
            filter: filter, history: flaw == .dropsPresenceHistory ? false : history, onChange: onChange)
    }

    func declarePresence(key: String) throws(ZenohClientError) -> any ZenohHold {
        let hold = try inner.declarePresence(key: key)
        return flaw == .leavesItsTokenBehind ? StuckHold() : hold
    }

    func put(key: String, payload: [UInt8]) throws(ZenohPutError) {
        let sent = flaw == .truncatesPayload && !payload.isEmpty ? Array(payload.dropLast()) : payload
        try inner.put(key: key, payload: sent)
    }

    func onSessionEnd(_ onEnd: @escaping @Sendable (String) -> Void) throws(ZenohClientError) {
        if flaw == .neverTellsTheSessionEnded {
            return
        }
        try inner.onSessionEnd(onEnd)
    }

    func close() {
        if flaw == .leavesItsTokenBehind {
            return
        }
        inner.close()
    }
}

private struct FlawedHarness: ZenohDisruptable {
    let base = InMemoryZenohHarness()
    let flaw: Flaw

    func connect() throws(ZenohClientError) -> any ZenohClient {
        FlawedClient(try base.connect(), flaw)
    }

    func endSessions() {
        base.endSessions()
    }

    func restore() {
        base.restore()
    }
}

/// The scenario that must fail for `flaw`, with the patience a router in memory needs and no more, so a kit
/// that waits out its full bound on a broken client does not make this suite slow.
private func failureOf(
    _ flaw: Flaw, _ scenario: (FlawedHarness, Duration) throws(ZenohConformanceFailure) -> Void
) -> ZenohConformanceFailure? {
    do throws(ZenohConformanceFailure) {
        try scenario(FlawedHarness(flaw: flaw), .milliseconds(400))
        return nil
    } catch {
        return error
    }
}

@Test func aClientThatTruncatesAPayloadFailsTheScenarioOfTwoMachines() {
    let found = failureOf(.truncatesPayload) { (harness: FlawedHarness, patience: Duration) throws(ZenohConformanceFailure) -> Void in
        try ZenohConformance.twoMachinesMeet(harness, patience: patience)
    }
    #expect(found?.scenario == "two_machines_meet")
    #expect(found?.message.contains("arrived changed") == true, "\(String(describing: found))")
}

@Test func aClientThatDropsPresenceHistoryFailsTheScenarioOfTwoMachines() {
    let found = failureOf(.dropsPresenceHistory) { (harness: FlawedHarness, patience: Duration) throws(ZenohConformanceFailure) -> Void in
        try ZenohConformance.twoMachinesMeet(harness, patience: patience)
    }
    #expect(found?.scenario == "two_machines_meet")
    #expect(found?.message.contains("the phone to learn the server is up") == true, "\(String(describing: found))")
}

@Test func aClientThatLeavesItsTokenBehindFailsTheScenarioOfAMachineThatWentAway() {
    let found = failureOf(.leavesItsTokenBehind) { (harness: FlawedHarness, patience: Duration) throws(ZenohConformanceFailure) -> Void in
        try ZenohConformance.aMachineThatGoesAwayIsToldAsLost(harness, patience: patience)
    }
    #expect(found?.scenario == "a_machine_that_goes_away_is_told_as_lost")
    #expect(found?.message.contains("the hub to be told the guest is lost") == true, "\(String(describing: found))")
}

@Test func aClientThatNeverSaysItsSessionEndedFailsTheScenarioOfALostSession() {
    let found = failureOf(.neverTellsTheSessionEnded) { (harness: FlawedHarness, patience: Duration) throws(ZenohConformanceFailure) -> Void in
        try ZenohConformance.aMachineThatLosesItsSessionComesBackAndResyncs(harness, patience: patience)
    }
    #expect(found?.scenario == "a_machine_that_loses_its_session_comes_back_and_resyncs")
    #expect(found?.message.contains("because the session ended") == true, "\(String(describing: found))")
}
