// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Dispatch
import SceMesh
import Synchronization

// What every client of the Zenoh binding is held to (SCE_MESH.md section 20.4).
//
// The binding talks to its client through `ZenohClient`, so a client is a choice. This file is what a choice
// has to pass: scenarios that open machines through `ZenohInbox` on whatever client a harness connects, against
// a router the harness points it at, and say what the machines were told. A client in this repository and a
// client in another repository run the same functions, so "does the same four operations" is a test and not a
// claim.
//
// It holds the client's side and no more. The scenarios are read through the events a binding gives its core
// (`LinkEvent`): the bytes, the order, the presence and the end of a session are what a client can get wrong,
// and the engine above them is held by the core's own tests. A scenario that fails throws a
// `ZenohConformanceFailure`, so the file asks nothing of the test library a client uses:
//
//     @Test func twoMachinesMeet() throws {
//         try ZenohConformance.twoMachinesMeet(MyHarness(port: startRouter()))
//     }
//
// Nothing here is a timing claim: every wait has a generous bound and the assertions are on what arrived. The
// router, and how a harness starts or finds it, are the harness's. The scenarios name their machines
// ("server", "phone", "bystander", "hub", "device", "me", "alice"), so two of them must not run against one
// router at once, and a harness that wants none of one scenario's closed sessions seen by the next gives each
// a router of its own.

/// Where a client under test comes from: a way to connect one, each with a session of its own on the router
/// the harness points it at.
public protocol ZenohHarness: Sendable {
    func connect() throws(ZenohClientError) -> any ZenohClient
}

/// A harness that can take its router away and give it back: what the scenario of a lost session needs.
public protocol ZenohDisruptable: ZenohHarness {
    /// End every session the harness's clients hold, as a router restart does, and refuse `connect` until
    /// `restore` is called.
    func endSessions()

    /// The router answers again.
    func restore()
}

/// A scenario that did not hold: which, and what the machines were seen to do instead.
public struct ZenohConformanceFailure: Error, Equatable, CustomStringConvertible, Sendable {
    public let scenario: String
    public let message: String

    public init(scenario: String, message: String) {
        self.scenario = scenario
        self.message = message
    }

    public var description: String {
        "\(scenario): \(message)"
    }
}

public enum ZenohConformance {
    /// Two machines that only know each other's names learn each is up, from the router's presence plane and
    /// nothing they were told (section 20.3), and what one sends reaches the other under the sender's name,
    /// byte for byte, once, and in the order it was sent (sections 20.1 and 20.2); a machine addressed by no
    /// one is sent nothing.
    public static func twoMachinesMeet(
        _ harness: some ZenohHarness, patience: Duration = .seconds(10)
    ) throws(ZenohConformanceFailure) {
        let name = "two_machines_meet"
        let wait = Waiter(scenario: name, patience: patience)
        let serverTold = Watch(), phoneTold = Watch(), bystanderTold = Watch()
        let server = try open(harness, "server", serverTold, name)
        let phone = try open(harness, "phone", phoneTold, name)
        let bystander = try open(harness, "bystander", bystanderTold, name)
        defer { [server, phone, bystander].forEach { $0.close() } }

        try wait.until("the server to learn the phone is up") { serverTold.contains(.ready(peer: "phone")) }
        try wait.until("the phone to learn the server is up") { phoneTold.contains(.ready(peer: "server")) }

        // Every byte value, nothing, and a payload past what a small buffer holds: one envelope is one put.
        let payloads: [[UInt8]] = [
            [], [0], [255], Array(0...255), (0..<65_536).map { UInt8(truncatingIfNeeded: $0 &* 31 &+ 7) },
        ]
        for payload in payloads {
            try send(server, to: "phone", payload, name)
        }
        try wait.until("the phone to receive every envelope") {
            phoneTold.received(from: "server").count >= payloads.count
        }
        guard phoneTold.received(from: "server") == payloads else {
            throw failure(name, "what the server sent the phone arrived changed, twice or out of order")
        }

        try send(phone, to: "server", [1, 2, 3], name)
        try wait.until("the server to hear the phone back") { !serverTold.received(from: "phone").isEmpty }
        guard serverTold.received(from: "phone") == [[1, 2, 3]] else {
            throw failure(name, "the phone's answer arrived changed or twice")
        }

        try send(server, to: "bystander", [42], name)
        try wait.until("the bystander to receive what was sent it") { !bystanderTold.allReceived().isEmpty }
        guard bystanderTold.allReceived() == [[42]] else {
            throw failure(name, "the bystander was sent what was addressed to another machine")
        }
    }

    /// A machine whose binding is closed is told to the other as lost, by its presence token ending, and the
    /// binding keeps no keepalive of its own (section 20.3).
    public static func aMachineThatGoesAwayIsToldAsLost(
        _ harness: some ZenohHarness, patience: Duration = .seconds(10)
    ) throws(ZenohConformanceFailure) {
        let name = "a_machine_that_goes_away_is_told_as_lost"
        let wait = Waiter(scenario: name, patience: patience)
        let hubTold = Watch(), guestTold = Watch()
        let hub = try open(harness, "hub", hubTold, name)
        let guest = try open(harness, "guest", guestTold, name)
        defer { hub.close() }

        try wait.until("the hub to learn the guest is up") { hubTold.contains(.ready(peer: "guest")) }
        guest.close()
        try wait.until("the hub to be told the guest is lost") {
            hubTold.contains(.lost(peer: "guest", reason: "its presence token ended"))
        }
    }

    /// A device that is on only while its user presses sync: the hub stays up, the device comes, sends, goes,
    /// and comes back under the same name, and each edge is seen once: up, an envelope, lost, up again, an
    /// envelope again, and the device learns the hub is there from the presence history when it returns.
    public static func aDeviceThatWasAwayCatchesUpFromAHubThatStayedUp(
        _ harness: some ZenohHarness, patience: Duration = .seconds(10)
    ) throws(ZenohConformanceFailure) {
        let name = "a_device_that_was_away_catches_up_from_a_hub_that_stayed_up"
        let wait = Waiter(scenario: name, patience: patience)
        let hubTold = Watch()
        let hub = try open(harness, "hub", hubTold, name)
        defer { hub.close() }

        let firstTold = Watch()
        let first = try open(harness, "device", firstTold, name)
        try wait.until("the hub to learn the device is up") { hubTold.contains(.ready(peer: "device")) }
        try wait.until("the device to learn the hub is up") { firstTold.contains(.ready(peer: "hub")) }
        try send(first, to: "hub", [1], name)
        try wait.until("the hub to receive the first edit") { hubTold.received(from: "device").count == 1 }
        first.close()
        try wait.until("the hub to be told the device is lost") {
            hubTold.contains(.lost(peer: "device", reason: "its presence token ended"))
        }

        let secondTold = Watch()
        let second = try open(harness, "device", secondTold, name)
        defer { second.close() }
        try wait.until("the hub to see the device come back") { hubTold.count(.ready(peer: "device")) == 2 }
        try wait.until("the returned device to learn the hub is there") {
            secondTold.contains(.ready(peer: "hub"))
        }
        try send(second, to: "hub", [2], name)
        try wait.until("the hub to receive the second edit") { hubTold.received(from: "device").count == 2 }
        guard hubTold.received(from: "device") == [[1], [2]] else {
            throw failure(name, "the hub did not receive both edits, once each and in order")
        }
    }

    /// The router goes away and comes back, and the machines that reconnect are told every peer they had ready
    /// is lost, then learn from the presence history who is still there, so a machine that left during the gap
    /// is never believed to be up (section 20.3). It needs a client that tells the binding when its session
    /// ended (`ZenohClient.onSessionEnd`); a client that cannot says so and is not held to it.
    public static func aMachineThatLosesItsSessionComesBackAndResyncs(
        _ harness: some ZenohDisruptable, patience: Duration = .seconds(10)
    ) throws(ZenohConformanceFailure) {
        let name = "a_machine_that_loses_its_session_comes_back_and_resyncs"
        let wait = Waiter(scenario: name, patience: patience)
        let factory = HarnessFactory(harness)
        let quick = ZenohBackoff(initial: .milliseconds(20), max: .milliseconds(200), factor: 2, jitter: false)
        let meTold = Watch(), hubTold = Watch(), aliceTold = Watch()

        let me = try reopening(factory, "me", meTold, quick, name)
        defer { me.close() }
        let hub = try reopening(factory, "hub", hubTold, quick, name)
        defer { hub.close() }
        // Alice's binding is the plain one: when the router goes her session ends for good, which is a
        // machine that left during the gap.
        let alice = try open(harness, "alice", aliceTold, name)
        defer { alice.close() }

        try wait.until("me to learn hub and alice are up") {
            meTold.contains(.ready(peer: "hub")) && meTold.contains(.ready(peer: "alice"))
        }

        harness.endSessions()
        try wait.until("me to be told hub and alice are lost, because the session ended") {
            meTold.count(.lost(peer: "hub", reason: "the session to the router ended")) == 1
                && meTold.count(.lost(peer: "alice", reason: "the session to the router ended")) == 1
        }

        harness.restore()
        try wait.until("me to learn hub is there again") { meTold.count(.ready(peer: "hub")) == 2 }
        try wait.until("hub to learn me is there again") { hubTold.count(.ready(peer: "me")) == 2 }
        guard meTold.count(.ready(peer: "alice")) == 1 else {
            throw failure(name, "a machine that left during the gap was believed to be up after the resync")
        }
        try wait.until("a send to hub to be accepted on the new session") {
            me.transmit(peer: "hub", bytes: [9]) == nil
        }
        try wait.until("hub to receive it") { hubTold.received(from: "me") == [[9]] }
    }

    /// Every scenario, against a harness that can also end its sessions.
    public static func all(
        _ harness: some ZenohDisruptable, patience: Duration = .seconds(10)
    ) throws(ZenohConformanceFailure) {
        try twoMachinesMeet(harness, patience: patience)
        try aMachineThatGoesAwayIsToldAsLost(harness, patience: patience)
        try aDeviceThatWasAwayCatchesUpFromAHubThatStayedUp(harness, patience: patience)
        try aMachineThatLosesItsSessionComesBackAndResyncs(harness, patience: patience)
    }
}

// MARK: What the scenarios share

/// What a binding told its core, in order.
private final class Watch: Sendable {
    private let events = Mutex<[LinkEvent]>([])

    func add(_ event: LinkEvent) {
        events.withLock { $0.append(event) }
    }

    func contains(_ event: LinkEvent) -> Bool {
        events.withLock { $0.contains(event) }
    }

    func count(_ event: LinkEvent) -> Int {
        events.withLock { $0.filter { $0 == event }.count }
    }

    func received(from peer: String) -> [[UInt8]] {
        events.withLock { events in
            events.compactMap { event in
                if case let .received(from, bytes) = event, from == peer {
                    return bytes
                }
                return nil
            }
        }
    }

    func allReceived() -> [[UInt8]] {
        events.withLock { events in
            events.compactMap { event in
                if case let .received(_, bytes) = event {
                    return bytes
                }
                return nil
            }
        }
    }
}

/// The harness as the factory a reconnecting binding asks for a client each time it opens a session.
private final class HarnessFactory: ZenohClientFactory {
    private let harness: any ZenohHarness

    init(_ harness: some ZenohHarness) {
        self.harness = harness
    }

    func connect() throws(ZenohClientError) -> any ZenohClient {
        try harness.connect()
    }
}

private func failure(_ scenario: String, _ message: String) -> ZenohConformanceFailure {
    ZenohConformanceFailure(scenario: scenario, message: message)
}

private func open(
    _ harness: some ZenohHarness, _ machine: String, _ told: Watch, _ scenario: String
) throws(ZenohConformanceFailure) -> ZenohInbox {
    let client: any ZenohClient
    do throws(ZenohClientError) {
        client = try harness.connect()
    } catch {
        throw failure(scenario, "the harness could not connect a client for \(machine): \(error.message)")
    }
    do throws(ZenohError) {
        return try ZenohInbox.open(client: client, machine: machine) { told.add($0) }
    } catch {
        throw failure(scenario, "the binding could not be opened for \(machine): \(error)")
    }
}

private func reopening(
    _ factory: HarnessFactory, _ machine: String, _ told: Watch, _ backoff: ZenohBackoff, _ scenario: String
) throws(ZenohConformanceFailure) -> ZenohReconnectingInbox {
    do throws(ZenohError) {
        return try ZenohReconnectingInbox.open(factory: factory, machine: machine, backoff: backoff) {
            told.add($0)
        }
    } catch {
        throw failure(scenario, "the reconnecting binding could not be opened for \(machine): \(error)")
    }
}

private func send(
    _ transport: some Transport, to peer: String, _ bytes: [UInt8], _ scenario: String
) throws(ZenohConformanceFailure) {
    if let refused = transport.transmit(peer: peer, bytes: bytes) {
        throw failure(scenario, "a send to \(peer) was refused: \(refused.message ?? "no reason given")")
    }
}

/// Waiting in one scenario: how long it is patient for each thing it waits for.
private struct Waiter {
    let scenario: String
    let patience: Duration

    /// Wait until `condition` holds, polling, for at most `patience`.
    func until(_ what: String, _ condition: () -> Bool) throws(ZenohConformanceFailure) {
        let deadline = ContinuousClock.now + patience
        while !condition() {
            if ContinuousClock.now >= deadline {
                throw failure(scenario, "timed out waiting for \(what)")
            }
            let nap = DispatchSemaphore(value: 0)
            _ = nap.wait(timeout: .now() + .milliseconds(2))
        }
    }
}
