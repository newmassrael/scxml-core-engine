// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Dispatch
import Synchronization
import Testing

@testable import SceMesh

// The Zenoh binding of the host core (SCE_MESH.md section 20), held against the unit tests of the Rust core's
// `inbox` module, which is the reference (`backends/rust/mesh/src/inbox.rs`): a client that does nothing but
// remember how it was used is opened on, and each test states what the binding asked of it and what it told
// the core. The last three tests are the ones this core's own design adds: what the Swift binding promises
// about the order and the thread of what it delivers.

private struct Record {
    var calls: [String] = []
    var inbox: (@Sendable (String, [UInt8]) -> Void)?
    var presence: (@Sendable (String, ZenohPresence) -> Void)?
    var end: (@Sendable (String) -> Void)?

    /// How many calls had been made when the end notice was asked for.
    var endAskedAfter: Int?

    /// Whether the end notice itself is refused.
    var refuseEnd = false
    var puts: [(key: String, payload: [UInt8])] = []
    var refusePut: ZenohPutError?

    /// The declaration (`inbox`, `presence` or `token`) that fails.
    var failOn: String?
}

/// A hold whose undeclaration is logged.
private final class Undeclare: ZenohHold {
    let recorder: Recorder
    let what: String

    init(recorder: Recorder, what: String) {
        self.recorder = recorder
        self.what = what
    }

    func undeclare() {
        recorder.log("undeclare \(what)")
    }
}

/// A client that does nothing but remember how it was used.
private final class Recorder: ZenohClient {
    let record = Mutex(Record())

    func log(_ call: String) {
        record.withLock { $0.calls.append(call) }
    }

    var calls: [String] { record.withLock { $0.calls } }
    var puts: [(key: String, payload: [UInt8])] { record.withLock { $0.puts } }

    private func declare(_ what: String, _ detail: String) throws(ZenohClientError) -> any ZenohHold {
        let refused = record.withLock { record -> Bool in
            record.calls.append("declare \(what) \(detail)")
            return record.failOn == what
        }
        if refused {
            throw ZenohClientError("\(what) refused")
        }
        return Undeclare(recorder: self, what: what)
    }

    func subscribe(
        filter: String, onSample: @escaping @Sendable (String, [UInt8]) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        let hold = try declare("inbox", filter)
        record.withLock { $0.inbox = onSample }
        return hold
    }

    func subscribePresence(
        filter: String, history: Bool, onChange: @escaping @Sendable (String, ZenohPresence) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        let hold = try declare("presence", "\(filter) history=\(history)")
        record.withLock { $0.presence = onChange }
        return hold
    }

    func declarePresence(key: String) throws(ZenohClientError) -> any ZenohHold {
        try declare("token", key)
    }

    func put(key: String, payload: [UInt8]) throws(ZenohPutError) {
        let refusal = record.withLock { record -> ZenohPutError? in
            record.puts.append((key, payload))
            return record.refusePut
        }
        if let refusal {
            throw refusal
        }
    }

    func onSessionEnd(_ onEnd: @escaping @Sendable (String) -> Void) throws(ZenohClientError) {
        let refused = record.withLock { record -> Bool in
            record.endAskedAfter = record.calls.count
            if record.refuseEnd {
                return true
            }
            record.end = onEnd
            return false
        }
        if refused {
            throw ZenohClientError("end refused")
        }
    }

    func close() {
        log("close client")
    }

    /// The callback the binding gave the client, called as the client's thread would.
    func sample(_ key: String, _ payload: [UInt8]) {
        let callback = record.withLock { $0.inbox }
        callback?(key, payload)
    }

    func presence(_ key: String, _ kind: ZenohPresence) {
        let callback = record.withLock { $0.presence }
        callback?(key, kind)
    }

    func endSession(_ words: String) {
        let callback = record.withLock { $0.end }
        callback?(words)
    }
}

/// What the binding told the core, in the order it told it.
private final class Collected: Sendable {
    private let events = Mutex<[LinkEvent]>([])

    func add(_ event: LinkEvent) {
        events.withLock { $0.append(event) }
    }

    /// Everything told since the last call.
    func drain() -> [LinkEvent] {
        events.withLock { events -> [LinkEvent] in
            let taken = events
            events.removeAll()
            return taken
        }
    }
}

private func openBinding(_ client: Recorder, machine: String = "me") throws(ZenohError) -> (ZenohInbox, Collected) {
    let collected = Collected()
    let binding = try ZenohInbox.open(client: client, machine: machine) { collected.add($0) }
    return (binding, collected)
}

private func lostOnEnd(_ peer: String) -> LinkEvent {
    .lost(peer: peer, reason: "the session to the router ended")
}

@Test func keysCarryTheReceiverThenTheSender() {
    #expect(ZenohInboxKeys.inbox(receiver: "server", sender: "phone-1") == "sce-mesh/1/to/server/phone-1")
    #expect(ZenohInboxKeys.inboxFilter(receiver: "server") == "sce-mesh/1/to/server/*")
    #expect(ZenohInboxKeys.presence(machine: "phone-1") == "sce-mesh/1/up/phone-1")
    #expect(ZenohInboxKeys.presenceFilter == "sce-mesh/1/up/*")
}

@Test func aMachineIsNamedByTheLastChunkOfItsKey() {
    #expect(ZenohInboxKeys.lastChunk(of: "sce-mesh/1/to/server/phone-1") == "phone-1")
    #expect(ZenohInboxKeys.lastChunk(of: "sce-mesh/1/up/server") == "server")
    #expect(ZenohInboxKeys.lastChunk(of: "") == "")
}

@Test func aNameThatIsNotAKeyChunkIsRefusedBeforeTheClientIsTouched() throws {
    for bad in ["", "a/b", "a*b", "a$b", "a b", "a?b", "a#b"] {
        let client = Recorder()
        #expect(throws: ZenohError.notAKeyChunk(bad)) {
            _ = try openBinding(client, machine: bad)
        }
        #expect(client.calls == [], "\(bad) declared something")
    }
    try checkZenohMachineName("phone-1")
}

@Test func theInboxIsSubscribedBeforePresenceIsDeclared() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)
    #expect(
        client.calls == [
            "declare inbox sce-mesh/1/to/me/*",
            "declare presence sce-mesh/1/up/* history=true",
            "declare token sce-mesh/1/up/me",
        ])
    binding.close()
}

@Test func aSampleOnTheInboxIsReceivedFromTheLastChunkOfItsKey() throws {
    let client = Recorder()
    let (binding, collected) = try openBinding(client)

    client.sample("sce-mesh/1/to/me/alice", Array("hello".utf8))
    client.sample("sce-mesh/1/to/me/a b", Array("unnamed".utf8))
    #expect(
        collected.drain() == [.received(peer: "alice", bytes: Array("hello".utf8))],
        "a sender that is not a key chunk is not an event")
    binding.close()
}

@Test func presenceIsReadyOrLostAndNeverOnesOwn() throws {
    let client = Recorder()
    let (binding, collected) = try openBinding(client)

    client.presence("sce-mesh/1/up/alice", .put)
    client.presence("sce-mesh/1/up/me", .put)
    client.presence("sce-mesh/1/up/me", .delete)
    client.presence("sce-mesh/1/up/a b", .put)
    client.presence("sce-mesh/1/up/alice", .delete)
    #expect(
        collected.drain() == [
            .ready(peer: "alice"),
            .lost(peer: "alice", reason: "its presence token ended"),
        ],
        "the machine's own presence and an unnamed one are not events")
    binding.close()
}

@Test func aTransmitIsOnePutOnThePeersInboxUnderThisMachinesName() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)

    #expect(binding.transmit(peer: "alice", bytes: Array("envelope".utf8)) == nil, "the put is accepted")
    let puts = client.puts
    #expect(puts.count == 1)
    #expect(puts.first?.key == "sce-mesh/1/to/alice/me")
    #expect(puts.first?.payload == Array("envelope".utf8))
    binding.close()
}

@Test func aTransmitToANameThatIsNotAKeyChunkIsTerminalAndNeverPut() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)

    let failure = binding.transmit(peer: "a/b", bytes: Array("envelope".utf8))
    #expect(failure?.retryable == false)
    #expect(client.puts.isEmpty)
    binding.close()
}

@Test func aRefusedPutCarriesTheClientsWordsAndItsRetryReading() throws {
    for retryable in [true, false] {
        let client = Recorder()
        let (binding, _) = try openBinding(client)
        client.record.withLock { $0.refusePut = ZenohPutError(retryable: retryable, message: "no route") }

        let failure = binding.transmit(peer: "alice", bytes: Array("envelope".utf8))
        #expect(failure == TransportFailure(retryable: retryable, message: "no route"))
        binding.close()
    }
}

@Test func aDeclarationTheClientRefusesFailsTheOpenAndReleasesWhatCameBefore() {
    let client = Recorder()
    client.record.withLock { $0.failOn = "token" }

    #expect(throws: ZenohError.session("token refused")) {
        _ = try openBinding(client)
    }
    let calls = client.calls
    for released in ["undeclare inbox", "undeclare presence", "close client"] {
        #expect(calls.contains(released), "\(released) is missing from \(calls)")
    }
}

@Test func closingTheBindingClosesTheClientFirstAndUndeclaresAllThreeInOrder() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)
    let opened = client.calls.count

    binding.close()
    binding.close()
    #expect(
        Array(client.calls[opened...]) == [
            "close client",
            "undeclare inbox",
            "undeclare presence",
            "undeclare token",
        ],
        "the client first, the holds in the order they were declared, and a second close nothing")
}

@Test func droppingTheBindingClosesIt() throws {
    let client = Recorder()
    do {
        let (binding, collected) = try openBinding(client)
        _ = (binding, collected)
    }
    let calls = client.calls
    for gone in ["close client", "undeclare inbox", "undeclare presence", "undeclare token"] {
        #expect(calls.contains(gone), "\(gone) is missing from \(calls)")
    }
}

@Test func theEndNoticeIsAskedForBeforeAnythingIsDeclared() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)
    #expect(client.record.withLock { $0.endAskedAfter } == 0)
    binding.close()
}

@Test func aClientThatRefusesTheEndNoticeFailsTheOpenBeforeDeclaring() {
    let client = Recorder()
    client.record.withLock { $0.refuseEnd = true }

    #expect(throws: ZenohError.session("end refused")) {
        _ = try openBinding(client)
    }
    #expect(client.calls == ["close client"])
}

@Test func theEndOfTheSessionTellsEveryReadyPeerLostOnce() throws {
    let client = Recorder()
    let (binding, collected) = try openBinding(client)

    client.presence("sce-mesh/1/up/alice", .put)
    client.presence("sce-mesh/1/up/bob", .put)
    client.presence("sce-mesh/1/up/carol", .put)
    // Bob left before the session ended, and neither the machine's own presence nor an unnamed one is a peer.
    client.presence("sce-mesh/1/up/bob", .delete)
    client.presence("sce-mesh/1/up/me", .put)
    client.presence("sce-mesh/1/up/a b", .put)
    #expect(binding.sessionEnded == nil)

    client.endSession("link reset")
    #expect(
        collected.drain() == [
            .ready(peer: "alice"),
            .ready(peer: "bob"),
            .ready(peer: "carol"),
            .lost(peer: "bob", reason: "its presence token ended"),
            lostOnEnd("alice"),
            lostOnEnd("carol"),
        ],
        "each peer ready at the end is lost once, bob who had left is not lost again, and dave who was never seen is not lost at all"
    )
    #expect(binding.sessionEnded == "link reset")
    binding.close()
}

@Test func presenceAfterTheEndTellsTheCoreNothing() throws {
    let client = Recorder()
    let (binding, collected) = try openBinding(client)

    client.endSession("link reset")
    client.presence("sce-mesh/1/up/alice", .put)
    client.presence("sce-mesh/1/up/alice", .delete)
    #expect(
        collected.drain().isEmpty,
        "a client that carries on by itself does not make peers ready on an ended binding")
    binding.close()
}

@Test func aTransmitAfterTheEndIsARetryableFailureAndPutsNothing() throws {
    let client = Recorder()
    let (binding, _) = try openBinding(client)

    client.endSession("link reset")
    let failure = binding.transmit(peer: "alice", bytes: Array("envelope".utf8))
    #expect(failure == TransportFailure(retryable: true, message: "the session to the router has ended"))
    #expect(client.puts.isEmpty, "the client was not asked")
    binding.close()
}

@Test func aSecondEndChangesNothing() {
    var state = ZenohSessionState()
    _ = state.presence(of: "alice", .put)

    #expect(state.end("first") == ["alice"], "the first end counts")
    #expect(state.end("second") == nil, "a second end tells nobody")
    #expect(state.endedWords == "first")
    #expect(state.ready.isEmpty, "the end leaves no peer ready")
}

// What this core's design promises beyond the Rust reference: the sink is called outside the binding's lock,
// one thread at a time, and in the order the state changed.

@Test func aSinkThatCallsBackIntoTheBindingDoesNotDeadlockIt() throws {
    let client = Recorder()
    let box = Mutex<ZenohInbox?>(nil)
    let answers = Mutex<[TransportFailure?]>([])
    let binding = try ZenohInbox.open(client: client, machine: "me") { event in
        // The core reacts to a peer being ready by sending to it, from inside the event.
        if case let .ready(peer) = event, let inbox = box.withLock({ $0 }) {
            let answer = inbox.transmit(peer: peer, bytes: [1])
            answers.withLock { $0.append(answer) }
        }
    }
    box.withLock { $0 = binding }

    client.presence("sce-mesh/1/up/alice", .put)
    #expect(answers.withLock { $0 } == [nil], "the send from inside the event was made and accepted")
    #expect(client.puts.count == 1)
    box.withLock { $0 = nil }
    binding.close()
}

@Test func anEventIsNeverDeliveredWhileAnotherIsBeingDelivered() throws {
    let client = Recorder()
    let inside = Atomic<Int>(0)
    let overlaps = Atomic<Int>(0)
    let delivered = Atomic<Int>(0)
    let binding = try ZenohInbox.open(client: client, machine: "me") { _ in
        if inside.add(1, ordering: .sequentiallyConsistent).newValue != 1 {
            overlaps.add(1, ordering: .sequentiallyConsistent)
        }
        // Long enough for a second thread to arrive while this one is still in the sink.
        for _ in 0..<200 { _ = delivered.load(ordering: .relaxed) }
        delivered.add(1, ordering: .sequentiallyConsistent)
        inside.subtract(1, ordering: .sequentiallyConsistent)
    }

    DispatchQueue.concurrentPerform(iterations: 8) { thread in
        for index in 0..<50 {
            client.presence("sce-mesh/1/up/peer-\(thread)-\(index)", .put)
        }
    }
    #expect(overlaps.load(ordering: .sequentiallyConsistent) == 0, "the sink was entered by one thread at a time")
    #expect(delivered.load(ordering: .sequentiallyConsistent) == 400, "and every event reached it")
    binding.close()
}

@Test func whatTheCoreIsToldIsInTheOrderTheStateChanged() throws {
    let client = Recorder()
    let (binding, collected) = try openBinding(client)

    // One thread makes a peer ready and then lost, and the end of the session comes from another: whatever
    // the interleaving, a peer is never told ready after it was told lost on the end.
    DispatchQueue.concurrentPerform(iterations: 2) { thread in
        if thread == 0 {
            for index in 0..<100 {
                client.presence("sce-mesh/1/up/peer-\(index)", .put)
            }
        } else {
            client.endSession("link reset")
        }
    }
    var lostOnEndSeen: Set<String> = []
    for event in collected.drain() {
        switch event {
        case let .ready(peer):
            #expect(!lostOnEndSeen.contains(peer), "\(peer) was told ready after it was told lost")
        case let .lost(peer, reason) where reason == "the session to the router ended":
            lostOnEndSeen.insert(peer)
        default:
            break
        }
    }
    binding.close()
}
