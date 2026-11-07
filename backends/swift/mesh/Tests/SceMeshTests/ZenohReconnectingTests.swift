// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization
import Testing

import SceMeshZenohKit

@testable import SceMesh

// The binding that opens itself again (SCE_MESH.md section 20.3, `open_reconnecting` of the Rust core,
// `backends/rust/mesh/src/reconnect.rs`), driven on a clock of its own: no test here sleeps, and every wait is
// one the binding asked the scheduler for.

private final class Told: Sendable {
    private let events = Mutex<[LinkEvent]>([])

    func add(_ event: LinkEvent) {
        events.withLock { $0.append(event) }
    }

    var all: [LinkEvent] { events.withLock { $0 } }
}

/// A machine kept open on the router through a factory, on a scheduler that moves when the test says.
private struct Rig {
    let router: InMemoryZenohRouter
    let factory: InMemoryZenohFactory
    let scheduler: ManualZenohScheduler
    let binding: ZenohReconnectingInbox
    let told: Told
}

private let noJitter = ZenohBackoff(jitter: false)

private func makeRig(
    _ router: InMemoryZenohRouter = InMemoryZenohRouter(), machine: String = "me",
    backoff: ZenohBackoff = noJitter, draw: @escaping @Sendable () -> Double = { 0.5 }
) throws(ZenohError) -> Rig {
    let factory = InMemoryZenohFactory(router: router)
    let scheduler = ManualZenohScheduler()
    let told = Told()
    let binding = try ZenohReconnectingInbox.open(
        factory: factory, machine: machine, backoff: backoff, scheduler: scheduler, random: draw
    ) { told.add($0) }
    return Rig(router: router, factory: factory, scheduler: scheduler, binding: binding, told: told)
}

private struct Peer {
    let client: InMemoryZenohClient
    let binding: ZenohInbox
}

private func peer(_ router: InMemoryZenohRouter, _ name: String) throws(ZenohError) -> Peer {
    let client = router.connect()
    return Peer(client: client, binding: try ZenohInbox.open(client: client, machine: name) { _ in })
}

// MARK: The policy

@Test func theWaitDoublesFromTheInitialToTheCapAndStaysThere() {
    let backoff = ZenohBackoff(initial: .milliseconds(250), max: .seconds(30), factor: 2, jitter: false)
    let waits = (0..<9).map { backoff.baseDelay(attempt: $0) }
    #expect(
        waits == [
            .milliseconds(250), .milliseconds(500), .seconds(1), .seconds(2), .seconds(4), .seconds(8),
            .seconds(16), .seconds(30), .seconds(30),
        ])
    #expect(backoff.baseDelay(attempt: 1_000) == .seconds(30), "a long run of failures does not grow past the cap")
}

@Test func theBackoffReadsOddSettingsTheWayTheRustCoreDoes() {
    #expect(ZenohBackoff(initial: .seconds(60), max: .seconds(30)).baseDelay(attempt: 0) == .seconds(30), "initial above max")
    #expect(
        ZenohBackoff(initial: .seconds(1), max: .seconds(30), factor: 0).baseDelay(attempt: 5) == .seconds(1),
        "a factor below 1 is read as 1")
    #expect(
        ZenohBackoff(initial: .zero, max: .seconds(30)).baseDelay(attempt: 5) == .zero, "a zero wait stays zero")
    #expect(
        ZenohBackoff(initial: .milliseconds(1), max: .seconds(30), factor: Int.max).baseDelay(attempt: 1) == .seconds(30),
        "a factor too large to multiply by is the cap")
}

@Test func jitterSpreadsTheWaitByTwentyPercentEitherWayAndNoMore() {
    let backoff = ZenohBackoff(initial: .seconds(10), max: .seconds(30), factor: 2, jitter: true)
    #expect(backoff.delay(attempt: 0, draw: 0) == .seconds(8))
    #expect(backoff.delay(attempt: 0, draw: 0.5) == .seconds(10))
    #expect(backoff.delay(attempt: 0, draw: 1) == .seconds(12))
    #expect(backoff.delay(attempt: 0, draw: -3) == .seconds(8), "a draw below 0 is 0")
    #expect(backoff.delay(attempt: 0, draw: 9) == .seconds(12), "a draw above 1 is 1")
    #expect(
        ZenohBackoff(initial: .seconds(10), max: .seconds(30), jitter: false).delay(attempt: 0, draw: 1) == .seconds(10),
        "no jitter is the base wait exactly")
}

// MARK: The binding

@Test func aFirstConnectionThatFailsIsTheOpensErrorAndSchedulesNothing() {
    let router = InMemoryZenohRouter()
    let factory = InMemoryZenohFactory(router: router)
    factory.setDown(true)
    let scheduler = ManualZenohScheduler()

    #expect(throws: ZenohError.session("the router does not answer")) {
        _ = try ZenohReconnectingInbox.open(factory: factory, machine: "me", scheduler: scheduler) { _ in }
    }
    #expect(scheduler.pending == 0, "an address that does not answer is not a silent retry")
    #expect(factory.attempts == 1)
}

@Test func aNameThatIsNotAKeyChunkIsRefusedBeforeTheFactoryIsAsked() {
    let factory = InMemoryZenohFactory(router: InMemoryZenohRouter())
    #expect(throws: ZenohError.notAKeyChunk("a b")) {
        _ = try ZenohReconnectingInbox.open(factory: factory, machine: "a b") { _ in }
    }
    #expect(factory.attempts == 0)
}

@Test func theEndOfASessionIsFollowedByTheLossesThenTheNextSessionsReadiness() throws {
    let rig = try makeRig()
    let alice = try peer(rig.router, "alice")
    rig.router.settle()
    #expect(rig.told.all == [.ready(peer: "alice")])

    rig.router.drop(rig.factory.latest!, words: "link reset")
    rig.router.settle()
    #expect(rig.scheduler.scheduledDelays == [.milliseconds(250)], "one wait is asked for, the first of the backoff")
    #expect(
        rig.told.all == [.ready(peer: "alice"), .lost(peer: "alice", reason: "the session to the router ended")],
        "the core is told every peer it had ready is lost, before anything of the next session")

    rig.scheduler.advance(by: .milliseconds(249))
    #expect(rig.factory.attempts == 1, "the wait has not run out")
    rig.scheduler.advance(by: .milliseconds(1))
    rig.router.settle()
    #expect(rig.factory.attempts == 2)
    #expect(
        rig.told.all == [
            .ready(peer: "alice"),
            .lost(peer: "alice", reason: "the session to the router ended"),
            .ready(peer: "alice"),
        ], "the next session's history announces who is still there, after the losses")
    withExtendedLifetime((rig.binding, alice)) {}
}

@Test func aSendWhileThereIsNoSessionIsARetryableFailureAndNeverWaits() throws {
    let rig = try makeRig()
    let alice = try peer(rig.router, "alice")
    rig.router.settle()

    #expect(rig.binding.transmit(peer: "alice", bytes: [1]) == nil, "while the session is up the send goes out")
    rig.router.drop(rig.factory.latest!, words: "link reset")
    rig.router.settle()
    #expect(
        rig.binding.transmit(peer: "alice", bytes: [2])
            == TransportFailure(retryable: true, message: "the session to the router is being reopened"))

    rig.scheduler.advance(by: .milliseconds(250))
    #expect(rig.binding.transmit(peer: "alice", bytes: [3]) == nil, "and after the reopening it goes out again")
    withExtendedLifetime((rig.binding, alice)) {}
}

@Test func aFailedAttemptWaitsLongerEachTimeAndTheSuccessStartsTheBackoffOver() throws {
    let rig = try makeRig()
    rig.router.drop(rig.factory.latest!, words: "link reset")
    rig.router.settle()
    rig.factory.setDown(true)

    rig.scheduler.advance(by: .milliseconds(250))  // the first attempt: refused
    rig.scheduler.advance(by: .milliseconds(500))  // the second: refused
    rig.scheduler.advance(by: .seconds(1))  // the third: refused
    #expect(rig.factory.attempts == 4, "the connection that opened the binding and three refused ones")
    #expect(rig.scheduler.scheduledDelays == [.milliseconds(250), .milliseconds(500), .seconds(1), .seconds(2)])

    rig.factory.setDown(false)
    rig.scheduler.advance(by: .seconds(2))
    #expect(rig.factory.attempts == 5)
    #expect(rig.scheduler.pending == 0, "the session is back and nothing more is waited for")

    rig.router.drop(rig.factory.latest!, words: "link reset again")
    rig.router.settle()
    #expect(
        rig.scheduler.scheduledDelays.last == .milliseconds(250), "after a session was made the backoff starts over")
    withExtendedLifetime(rig.binding) {}
}

@Test func aPeerThatLeftWhileTheSessionWasDownIsNotReadyAfterTheReopening() throws {
    let rig = try makeRig()
    let alice = try peer(rig.router, "alice")
    rig.router.settle()

    rig.router.drop(rig.factory.latest!, words: "link reset")
    alice.binding.close()  // alice leaves while this machine has no session
    rig.router.settle()
    rig.scheduler.advance(by: .milliseconds(250))
    rig.router.settle()

    #expect(
        rig.told.all == [.ready(peer: "alice"), .lost(peer: "alice", reason: "the session to the router ended")],
        "everyone was lost first, and alice is not announced again because she is not there")
    withExtendedLifetime(rig.binding) {}
}

@Test func aJitteredWaitIsTheDrawsReadingOfTheBackoff() throws {
    let rig = try makeRig(backoff: ZenohBackoff(initial: .seconds(10), max: .seconds(30), jitter: true), draw: { 1 })
    rig.router.drop(rig.factory.latest!, words: "link reset")
    rig.router.settle()
    #expect(rig.scheduler.scheduledDelays == [.seconds(12)])
    withExtendedLifetime(rig.binding) {}
}

@Test func closingStopsTheReopeningAndEndsThisMachinesPresence() throws {
    let rig = try makeRig()
    rig.router.settle()
    #expect(rig.router.tokens == ["sce-mesh/1/up/me"])

    rig.binding.close()
    rig.router.settle()
    #expect(rig.router.tokens.isEmpty, "the session was closed, so the token ended")

    rig.binding.close()  // a second close does nothing
    #expect(rig.binding.transmit(peer: "x", bytes: [1])?.retryable == true)
}

@Test func closingWhileAWaitIsPendingTakesTheWaitBack() throws {
    let rig = try makeRig()
    rig.router.drop(rig.factory.latest!, words: "link reset")
    rig.router.settle()
    #expect(rig.scheduler.pending == 1)

    rig.binding.close()
    #expect(rig.scheduler.pending == 0, "the wait was cancelled")
    rig.scheduler.advance(by: .seconds(60))
    #expect(rig.factory.attempts == 1, "and no session was opened after the binding was closed")
}

@Test func aSessionTheClientEndsTwiceIsReopenedOnce() throws {
    let rig = try makeRig()
    let client = rig.factory.latest!
    rig.router.drop(client, words: "link reset")
    rig.router.drop(client, words: "link reset again")
    rig.router.settle()
    #expect(rig.scheduler.pending == 1, "one wait for one end")
    withExtendedLifetime(rig.binding) {}
}
