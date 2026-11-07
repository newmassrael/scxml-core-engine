// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Dispatch
import Synchronization
import Testing

import SceMeshZenohKit

@testable import SceMesh

// Several machines on one in-memory Zenoh router (SceMeshZenohKit), each with the binding of SCE_MESH.md
// section 20 on a client of its own: what the unit tests of the binding say it asks of a client, said here
// of the whole, with the router doing what a router does. The Rust core's `zenoh_through_a_router` is the
// reference, run there against a real router.

/// What one machine's binding told its core, in order.
private final class Told: Sendable {
    private let events = Mutex<[LinkEvent]>([])

    func add(_ event: LinkEvent) {
        events.withLock { $0.append(event) }
    }

    var all: [LinkEvent] { events.withLock { $0 } }
}

private struct Machine {
    let name: String
    let client: InMemoryZenohClient
    let binding: ZenohInbox
    let told: Told

    func send(to peer: String, _ bytes: [UInt8]) -> TransportFailure? {
        binding.transmit(peer: peer, bytes: bytes)
    }
}

private func start(_ router: InMemoryZenohRouter, _ name: String) throws(ZenohError) -> Machine {
    let client = router.connect()
    let told = Told()
    let binding = try ZenohInbox.open(client: client, machine: name) { told.add($0) }
    return Machine(name: name, client: client, binding: binding, told: told)
}

@Test func aStarMatchesOneChunkAndOnlyOne() {
    let cases: [(filter: String, key: String, matches: Bool)] = [
        ("sce-mesh/1/to/me/*", "sce-mesh/1/to/me/alice", true),
        ("sce-mesh/1/to/me/*", "sce-mesh/1/to/me/a/b", false),
        ("sce-mesh/1/to/me/*", "sce-mesh/1/to/me", false),
        ("sce-mesh/1/to/me/*", "sce-mesh/1/to/you/alice", false),
        ("sce-mesh/1/up/*", "sce-mesh/1/up/alice", true),
        ("sce-mesh/1/up/*", "sce-mesh/1/to/me/alice", false),
        ("a/b", "a/b", true),
        ("a/b", "a/c", false),
    ]
    for item in cases {
        #expect(
            InMemoryZenohRouter.matches(filter: item.filter, key: item.key) == item.matches,
            "\(item.filter) against \(item.key)")
    }
}

@Test func aMachineThatOpensAfterAnotherStillLearnsItIsUp() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    router.settle()
    #expect(a.told.all.isEmpty, "alone, and a machine is never told its own presence")

    let b = try start(router, "b")
    router.settle()
    #expect(a.told.all == [.ready(peer: "b")], "a sees b come up")
    #expect(b.told.all == [.ready(peer: "a")], "b, which opened later, learns a was up through the history")
    #expect(router.tokens == ["sce-mesh/1/up/a", "sce-mesh/1/up/b"])
    withExtendedLifetime((a, b)) {}
}

@Test func aSendReachesTheMachineItNamesAndNoOther() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    let c = try start(router, "c")
    router.settle()

    #expect(a.send(to: "b", Array("hello".utf8)) == nil)
    router.settle()
    #expect(b.told.all.contains(.received(peer: "a", bytes: Array("hello".utf8))))
    for other in [a, c] {
        let received = other.told.all.filter {
            if case .received = $0 { return true }
            return false
        }
        #expect(received.isEmpty, "\(other.name) was sent nothing")
    }
    withExtendedLifetime((a, b, c)) {}
}

@Test func aMachineThatLeavesIsLostToTheOthersByItsTokenEnding() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    router.settle()

    b.binding.close()
    router.settle()
    #expect(
        a.told.all == [.ready(peer: "b"), .lost(peer: "b", reason: "its presence token ended")],
        "the network reports a machine that went, and the binding keeps no keepalive of its own")
    #expect(router.tokens == ["sce-mesh/1/up/a"])
    withExtendedLifetime(a) {}
}

@Test func theEndOfASessionIsTheMachineGoingAwayAndTheCoreToldItsPeersAreLost() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    router.settle()

    router.drop(b.client, words: "link reset")
    router.settle()
    #expect(a.told.all.last == .lost(peer: "b", reason: "its presence token ended"), "a sees b's token end")
    #expect(
        b.told.all == [
            .ready(peer: "a"),
            .lost(peer: "a", reason: "the session to the router ended"),
        ], "b is told every peer it had ready is lost, once")
    #expect(b.binding.sessionEnded == "link reset")
    #expect(
        b.send(to: "a", [1]) == TransportFailure(retryable: true, message: "the session to the router has ended"),
        "a send after the end is a retryable failure and reaches nobody")
    withExtendedLifetime((a, b)) {}
}

@Test func aMachineThatDepartedDuringTheGapIsNotReadyAfterTheResync() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    router.settle()

    // b's session goes, and while b is without one a leaves: nothing tells b who left in the meantime.
    router.drop(b.client, words: "link reset")
    a.binding.close()
    router.settle()

    // b dials again. The fresh session's history announces only the machines that are still there.
    let again = try start(router, "b")
    let c = try start(router, "c")
    router.settle()
    #expect(
        again.told.all == [.ready(peer: "c")],
        "a left during the gap and is not announced; b was told it was lost when the old session ended")
    #expect(c.told.all == [.ready(peer: "b")])
    withExtendedLifetime((a, again, c)) {}
}

@Test func aMachineThatComesBackIsReadyAgain() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    router.settle()

    router.drop(b.client, words: "link reset")
    router.settle()
    // The machine that comes back is kept until the end: dropping a binding closes it.
    let again = try start(router, "b")
    router.settle()
    #expect(
        a.told.all == [
            .ready(peer: "b"),
            .lost(peer: "b", reason: "its presence token ended"),
            .ready(peer: "b"),
        ], "a sees b go and come back, each edge once")
    withExtendedLifetime((a, again)) {}
}

@Test func aSendOnASessionTheClientClosedItselfIsARetryableFailure() throws {
    let router = InMemoryZenohRouter()
    let a = try start(router, "a")
    let b = try start(router, "b")
    router.settle()

    a.client.close()
    let failure = a.send(to: "b", [1])
    #expect(failure == TransportFailure(retryable: true, message: "the session is closed"))
    #expect(a.binding.sessionEnded == nil, "a client that closes its own session tells the binding nothing")
    withExtendedLifetime((a, b)) {}
}

@Test func everyMachineReachesEveryOtherExactlyOnceAndInOrderPerSender() throws {
    let router = InMemoryZenohRouter()
    let names = ["m0", "m1", "m2", "m3", "m4", "m5"]
    let machines = try names.map { name throws(ZenohError) in try start(router, name) }
    router.settle()

    let perPair = 20
    DispatchQueue.concurrentPerform(iterations: machines.count) { index in
        let sender = machines[index]
        for peer in names where peer != sender.name {
            for sequence in 0..<perPair {
                _ = sender.send(to: peer, [UInt8(sequence)])
            }
        }
    }
    router.settle()

    for receiver in machines {
        var next: [String: Int] = [:]
        for event in receiver.told.all {
            guard case let .received(peer, bytes) = event else {
                continue
            }
            let expected = next[peer, default: 0]
            #expect(bytes == [UInt8(expected)], "\(peer) to \(receiver.name): envelope \(expected) out of order or twice")
            next[peer] = expected + 1
        }
        for peer in names where peer != receiver.name {
            #expect(next[peer] == perPair, "\(receiver.name) received \(next[peer] ?? 0) from \(peer)")
        }
    }
}
