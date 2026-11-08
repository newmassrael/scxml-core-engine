// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host half, case for case with the Kotlin and Rust cores' endpoint tests.

import SceMesh
import SceRuntime
import Testing

/// A transport that records what it sent and fails as told.
private final class Recorder: Transport {
    var sent: [(peer: String, bytes: [UInt8])] = []
    var failures: [TransportFailure] = []

    func transmit(peer: String, bytes: [UInt8]) -> TransportFailure? {
        if !failures.isEmpty {
            return failures.removeFirst()
        }
        sent.append((peer, bytes))
        return nil
    }
}

/// A clock the test moves, ids that count, and no jitter.
private final class Fixed: Environment {
    var now: Int64 = 0
    private var ids = 0

    func nowMs() -> Int64 { now }

    /// The wall clock a fixed distance ahead of the monotonic one, so a wire deadline and a kept one
    /// can be told apart.
    func nowUnixMs() -> Int64 { 1_000_000 + now }

    func envelopeID() -> [UInt8] {
        ids += 1
        return [UInt8](repeating: UInt8(truncatingIfNeeded: ids), count: 16)
    }

    func jitterDraw() -> Int64 { 0 }
}

private final class Seal: Sealer {
    let decide: (String, String, String) -> Sealed

    init(_ decide: @escaping (String, String, String) -> Sealed) {
        self.decide = decide
    }

    func seal(target: String, event: String, data: String) -> Sealed {
        decide(target, event, data)
    }
}

private final class AdmissionStub: EventAdmission {
    let decide: (String, String, String) -> Admit

    init(_ decide: @escaping (String, String, String) -> Admit) {
        self.decide = decide
    }

    func admit(peer: String, event: String, data: String) -> Admit {
        decide(peer, event, data)
    }
}

private final class Host {
    let transport = Recorder()
    let environment = Fixed()
    let endpoint: Endpoint

    init(machine: String, peer: String, retry: RetryPolicy?) {
        let router = Router(machine: machine, dedupWindow: 8, gapTimeoutMs: 50)
        router.addPeer(
            peer,
            config: PeerConfig(
                transport: "wss",
                buffer: OutboundBuffer(maxPending: 4, maxAgeMs: 0),
                retry: retry,
                stampSequence: false,
                delivery: Delivery(dedup: true, ordered: false),
                responders: [peer],
                deadlineMs: nil,
                replyEvents: []
            ))
        endpoint = Endpoint(router: router, transport: transport, environment: environment)
        endpoint.peerReady(peer)
    }

    /// The last transmission the transport took, which the macro of `#require` cannot take itself:
    /// it hands its argument over as an immutable value.
    func popSent() throws -> (peer: String, bytes: [UInt8]) {
        guard let sent = transport.sent.popLast() else {
            throw Unexpected(description: "the transport was handed nothing")
        }
        return sent
    }

    /// The events the endpoint queued, which in these tests are all it queued.
    func takeEvents() throws -> [EngineEvent] {
        try endpoint.takeCalls().map { call in
            guard case let .raise(event) = call else {
                throw Unexpected(description: "expected only events, got \(call)")
            }
            return event
        }
    }
}

@Suite struct EndpointTests {
    private func send(_ event: String, _ sendID: String) -> HostSendRequest {
        sendRequest("#hmi", event, #"{"a":1}"#, sendID: sendID)
    }

    private let reset = TransportFailure(retryable: true, message: "reset")

    @Test func aDeliveredEventCarriesTheFieldsTheCppCoreGivesIt() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        let hmi = Host(machine: "hmi", peer: "ecu", retry: nil)
        ecu.endpoint.send(send("go", "s-7"))
        let sent = try ecu.popSent()
        #expect(sent.peer == "hmi")
        hmi.endpoint.receive(peer: "ecu", bytes: sent.bytes)

        let events = try hmi.takeEvents()
        #expect(events.count == 1)
        let event = try #require(events.first)
        #expect(event.name == "go")
        #expect(event.metadata.data == #"{"a":1}"#)
        #expect(event.metadata.eventType == .externalEvent)
        #expect(event.metadata.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://ecu"))
        #expect(event.metadata.sender == nil, "no session sent it")
        #expect(event.metadata.sendID == "s-7")
    }

    @Test func aLinkMayBeForbiddenEvents() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        ecu.endpoint.send(send("go", "s-1"))
        let go = try ecu.popSent().bytes
        ecu.endpoint.send(send("stop", "s-2"))
        let stop = try ecu.popSent().bytes

        let hmi = Host(machine: "hmi", peer: "ecu", retry: nil)
        hmi.endpoint.forbidEvents(from: "ecu", ["go"])
        hmi.endpoint.receive(peer: "ecu", bytes: go)
        hmi.endpoint.receive(peer: "ecu", bytes: stop)
        #expect(try hmi.takeEvents().map(\.name) == ["stop"])
        #expect(hmi.endpoint.takeHostErrors() == [.eventForbidden(peer: "ecu", event: "go")])

        hmi.endpoint.forbidEvents(from: "ecu", [String]())
        let again = Host(machine: "hmi", peer: "ecu", retry: nil)
        again.endpoint.receive(peer: "ecu", bytes: go)
        #expect(try again.takeEvents().count == 1)
        #expect(again.endpoint.takeHostErrors().isEmpty)
    }

    /// What decides an event's admission is shown the link, the name and the data the sender wrote; one
    /// it does not admit never reaches the machine and the host is told why, one it admits goes through
    /// with what the sealer wrote taken out again.
    @Test func anAdmissionDecidesWhatReachesTheMachine() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        ecu.endpoint.setSealer(
            Seal { target, event, data in
                #expect(target == "#hmi")
                return event == "never" ? .withhold(reason: "this one does not leave") : .leave(data: data + "+seal")
            })
        ecu.endpoint.send(send("go", "s-1"))
        let go = try ecu.popSent().bytes
        ecu.endpoint.send(send("stop", "s-2"))
        let stop = try ecu.popSent().bytes

        let hmi = Host(machine: "hmi", peer: "ecu", retry: nil)
        hmi.endpoint.setAdmission(
            AdmissionStub { peer, event, data in
                #expect(peer == "ecu")
                if event == "go" {
                    return .allow(data: data.hasSuffix("+seal") ? String(data.dropLast(5)) : data)
                }
                return .refuse(reason: "\(event) is not for this link")
            })
        hmi.endpoint.receive(peer: "ecu", bytes: go)
        hmi.endpoint.receive(peer: "ecu", bytes: stop)
        let events = try hmi.takeEvents()
        #expect(events.map(\.name) == ["go"])
        #expect(events.first?.metadata.data == #"{"a":1}"#)
        #expect(
            hmi.endpoint.takeHostErrors()
                == [.eventNotAdmitted(peer: "ecu", event: "stop", reason: "stop is not for this link")])

        // A send the sealer says must not leave is dropped, told to the host, and nothing goes in its
        // place.
        ecu.endpoint.send(send("never", "s-3"))
        #expect(ecu.transport.sent.isEmpty)
        #expect(
            ecu.endpoint.takeHostErrors()
                == [.eventNotSealed(target: "#hmi", event: "never", reason: "this one does not leave")])
    }

    @Test func aSendWithoutAnIdArrivesWithoutOne() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        let hmi = Host(machine: "hmi", peer: "ecu", retry: nil)
        ecu.endpoint.send(send("go", ""))
        hmi.endpoint.receive(peer: "ecu", bytes: try ecu.popSent().bytes)
        #expect(try hmi.takeEvents().first?.metadata.sendID == "")
    }

    @Test func aFailedSendWithNoRetryPolicyIsLostAsSendFailed() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        ecu.transport.failures.append(reset)
        ecu.endpoint.send(send("go", "s-1"))

        let events = try ecu.takeEvents()
        #expect(events.count == 1)
        let event = try #require(events.first)
        #expect(event.name == "error.communication")
        #expect(
            event.metadata.data
                == #"{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss","transport_error":"reset"}"#
        )
        // Raised as the C++ core raises it: an envelope from its own machine.
        #expect(event.metadata.originAddress == "mesh://ecu")
        #expect(event.metadata.sendID == "")
    }

    @Test func aRetryableFailureIsSentAgainAfterItsBackoff() throws {
        let ecu = Host(
            machine: "ecu", peer: "hmi",
            retry: RetryPolicy(
                maxRetries: 2, initialBackoffMs: 100, backoffMultiplier: 2.0, maxBackoffMs: 1000, jitterPct: 0))
        ecu.transport.failures.append(reset)
        ecu.endpoint.send(send("go", "s-1"))
        #expect(ecu.transport.sent.isEmpty)
        #expect(try ecu.takeEvents().isEmpty)

        ecu.environment.now = 99
        ecu.endpoint.tick()
        #expect(ecu.transport.sent.isEmpty, "sent before its backoff")

        ecu.environment.now = 100
        ecu.endpoint.tick()
        #expect(ecu.transport.sent.count == 1)
        #expect(try ecu.takeEvents().isEmpty)
    }

    @Test func retriesThatRunOutAreDeliveryExhausted() throws {
        let ecu = Host(
            machine: "ecu", peer: "hmi",
            retry: RetryPolicy(
                maxRetries: 1, initialBackoffMs: 10, backoffMultiplier: 1.0, maxBackoffMs: 10, jitterPct: 0))
        ecu.transport.failures.append(reset)
        ecu.transport.failures.append(reset)
        ecu.endpoint.send(send("go", "s-1"))
        ecu.environment.now = 10
        ecu.endpoint.tick()

        #expect(
            try ecu.takeEvents().first?.metadata.data
                == #"{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"reset","attempts":2}"#
        )
    }

    @Test func aTerminalFailureIsNotRetried() throws {
        let ecu = Host(
            machine: "ecu", peer: "hmi",
            retry: RetryPolicy(
                maxRetries: 5, initialBackoffMs: 10, backoffMultiplier: 1.0, maxBackoffMs: 10, jitterPct: 0))
        ecu.transport.failures.append(TransportFailure(retryable: false, message: "reset"))
        ecu.endpoint.send(send("go", "s-1"))
        #expect(try ecu.takeEvents().first?.metadata.data.contains(#""attempts":1"#) == true)
        ecu.environment.now = 1000
        ecu.endpoint.tick()
        #expect(ecu.transport.sent.isEmpty)
    }

    @Test func whatOnlyTheHostCanActOnNeverReachesTheDocument() throws {
        let ecu = Host(machine: "ecu", peer: "hmi", retry: nil)
        ecu.endpoint.receive(peer: "stranger", bytes: [0xFF])
        #expect(try ecu.takeEvents().isEmpty)
        #expect(ecu.endpoint.takeHostErrors() == [.unknownPeer("stranger")])
    }

    /// docs/adr/0085, section 3: a send to a causal peer is answered with a verdict the host takes, and
    /// leaves only once the host reports its journal record committed; a link that comes back carries
    /// this copy's vector at once. Red if `apply` drops a verdict (none is taken), or drops
    /// `announceDue` (no vector leaves on the link's return).
    @Test func aCausalSendIsAVerdictAndLeavesOnceCommitted() throws {
        let transport = Recorder()
        let environment = Fixed()
        let router = Router(machine: "list", dedupWindow: 8, gapTimeoutMs: 50)
        router.addPeer(
            "people",
            config: PeerConfig(
                transport: "wss",
                buffer: nil,
                retry: nil,
                stampSequence: false,
                delivery: Delivery(dedup: false, ordered: false),
                responders: ["people"],
                deadlineMs: nil,
                replyEvents: [],
                causal: CausalGroup(name: "people", epoch: 0, view: ["phone/list", "laptop/list"])
            ))
        let endpoint = Endpoint(router: router, transport: transport, environment: environment)
        try endpoint.startCausal("people", start: .newIncarnation(causalReplica(1)), room: 1, announceEveryMs: 100)

        let dot = Dot(replica: causalReplica(1), seq: 1)
        endpoint.send(sendRequest("#people", "edit", #"{"n":1}"#))
        #expect(transport.sent.isEmpty, "nothing leaves before the journal commit")
        #expect(endpoint.takeVerdicts() == [CausalPeerVerdict(peer: "people", verdict: .accepted(dot))])
        endpoint.committed(peer: "people", dot: dot)
        #expect(transport.sent.count == 1)

        // The log's room is one change, which no peer has acknowledged yet: the next send is refused.
        endpoint.send(sendRequest("#people", "edit", #"{"n":2}"#))
        #expect(
            endpoint.takeVerdicts()
                == [CausalPeerVerdict(peer: "people", verdict: .refused(.noRoomForHistory))])
        #expect(transport.sent.count == 1)

        endpoint.peerReady("people")
        #expect(transport.sent.count == 2)
        let vector = try decodeEnvelope(try #require(transport.sent.last).bytes)
        #expect(vector.pattern == .causalVector)
        #expect(endpoint.takeHostErrors().isEmpty)
    }
}
