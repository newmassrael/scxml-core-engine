// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-18 mesh-18.1 mesh-18.2 mesh-18.3
//
// The SwiftNIO client of the WebSocket binding (SCE_MESH.md §mesh-18) against a real WebSocket
// server, the Swift twin of the Kotlin and Rust bindings' socket tests. It pins what the client puts
// on the wire and how it reports the link: the path that names the client (§mesh-18.1), one envelope
// per binary message and the 1003 close a text message earns (§mesh-18.2), and the ready, lost and
// keepalive edges (§mesh-18.3).

import NIOCore
import SceForgeRuntime
import SceMesh
import SceMeshWss
import SceRuntime
import Testing

/// A clock the test does not move, ids that count, and no jitter.
private final class Counting: Environment {
    private var now: Int64 = 0
    private var ids = 0

    func nowMs() -> Int64 {
        now += 1
        return now
    }

    func nowUnixMs() -> Int64 { now }

    func envelopeID() -> [UInt8] {
        ids += 1
        return [UInt8](repeating: UInt8(truncatingIfNeeded: ids), count: 16)
    }

    func jitterDraw() -> Int64 { 0 }
}

struct Unexpected: Error, CustomStringConvertible {
    var description: String
}

func router(machine: String, peer: String) -> Router {
    let router = Router(machine: machine, dedupWindow: 8, gapTimeoutMs: 50)
    router.addPeer(
        peer,
        config: PeerConfig(
            transport: "wss",
            buffer: OutboundBuffer(maxPending: 8, maxAgeMs: 0),
            retry: nil,
            stampSequence: false,
            delivery: Delivery(dedup: true, ordered: false),
            responders: [peer],
            deadlineMs: nil,
            replyEvents: []
        ))
    return router
}

func sendRequest(_ event: String, target: String = "#server", sendID: String = "s-1") -> HostSendRequest {
    HostSendRequest(
        processorType: meshProcessorType, eventName: event, target: target, sendID: sendID, eventData: #"{"v":1}"#)
}

@Suite(.serialized) struct WssClientTests {
    private func client(
        _ server: TestServer, events: Inbox<LinkEvent>, keepalive: TimeAmount = .seconds(30)
    ) throws -> WssClient {
        var configuration = WssConfiguration()
        configuration.keepalive = keepalive
        return try WssClient(
            base: "ws://127.0.0.1:\(server.port)", own: "client", server: "server", configuration: configuration
        ) { events.push($0) }
    }

    private func takeEvents(_ endpoint: Endpoint) throws -> [EngineEvent] {
        try endpoint.takeCalls().map { call in
            guard case let .raise(event) = call else {
                throw Unexpected(description: "expected only events, got \(call)")
            }
            return event
        }
    }

    @Test func envelopesCrossARealSocketBothWays() throws {
        let server = try TestServer()
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        let link = try client(server, events: events)
        let endpoint = Endpoint(router: router(machine: "client", peer: "server"), transport: link, environment: Counting())
        link.connect()

        let ready = try #require(events.next())
        #expect(ready == .ready(peer: "server"))
        #expect(server.paths.next() == "/sce-mesh/1/client")
        deliver(ready, to: endpoint)

        // Client to server: the core's envelope, as the socket carried it.
        endpoint.send(sendRequest("go"))
        let bytes = try #require(server.received.next())
        var cursor = SceCursor(bytes)
        let sent = try Envelope.decode(&cursor)
        #expect(sent.event_type == "go")
        #expect(sent.source == "client")
        #expect(sent.subject == "s-1")

        // Server to client: an envelope the server machine's core wrote.
        let serverRouter = router(machine: "server", peer: "client")
        guard case let .done(queued) = serverRouter.peerReady("client", nowMs: 0), queued.isEmpty else {
            throw Unexpected(description: "the server router queued something before it sent")
        }
        let routed = serverRouter.send(
            sendRequest("done", target: "#client", sendID: ""), id: [UInt8](repeating: 9, count: 16), nowMs: 0)
        guard case let .done(effects) = routed, effects.count == 1, case let .transmit(_, reply) = effects[0] else {
            throw Unexpected(description: "expected one transmission, got \(routed)")
        }
        try #require(server.sockets.next()).send(binary: reply)
        let received = try #require(events.next())
        guard case .received = received else {
            throw Unexpected(description: "expected a received message, got \(received)")
        }
        deliver(received, to: endpoint)

        let raised = try takeEvents(endpoint)
        #expect(raised.count == 1)
        #expect(raised.first?.name == "done")
        #expect(raised.first?.metadata.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://server"))
        #expect(raised.first?.metadata.sender == nil, "no session sent it")
        #expect(endpoint.takeHostErrors().isEmpty)
        link.close()
    }

    @Test func aRefusedUpgradeIsReportedAsALostLinkNotAReadyOne() throws {
        let server = try TestServer(refuseUpgrade: true)
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        try client(server, events: events).connect()
        let event = try #require(events.next())
        guard case let .lost(peer, reason) = event else {
            throw Unexpected(description: "expected a lost link, got \(event)")
        }
        #expect(peer == "server")
        #expect(reason.contains("404"), "\(reason)")
    }

    @Test func aDialThatFindsNobodyIsALostLink() throws {
        let server = try TestServer()
        let port = server.port
        server.shutdown()
        let events = Inbox<LinkEvent>()
        var configuration = WssConfiguration()
        configuration.connectTimeout = .seconds(2)
        try WssClient(base: "ws://127.0.0.1:\(port)", own: "client", server: "server", configuration: configuration) {
            events.push($0)
        }.connect()
        guard case .lost = try #require(events.next()) else {
            throw Unexpected(description: "expected a lost link")
        }
    }

    @Test func aClosedLinkEndsReadinessAndTheCoreRaisesRowOne() throws {
        let server = try TestServer()
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        let link = try client(server, events: events)
        let endpoint = Endpoint(router: router(machine: "client", peer: "server"), transport: link, environment: Counting())
        link.connect()
        deliver(try #require(events.next()), to: endpoint)

        try #require(server.sockets.next()).close()
        let lost = try #require(events.next())
        guard case .lost = lost else {
            throw Unexpected(description: "expected a lost link, got \(lost)")
        }
        deliver(lost, to: endpoint)
        let raised = try takeEvents(endpoint)
        #expect(
            raised.first?.metadata.data
                == #"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"server","transport":"wss"}"#
        )
    }

    @Test func aTextMessageIsAProtocolErrorClosedWithStatus1003() throws {
        let server = try TestServer()
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        try client(server, events: events).connect()
        #expect(events.next() == .ready(peer: "server"))

        // An envelope is a binary message; text is not one (§mesh-18.2).
        try #require(server.sockets.next()).send(text: "this is text, not an envelope")

        let lost = try #require(events.next())
        guard case let .lost(peer, _) = lost else {
            throw Unexpected(description: "expected a lost link, got \(lost)")
        }
        #expect(peer == "server")
        #expect(server.closeCodes.next() == 1003)
    }

    @Test func aSendBeforeTheLinkIsOpenIsARetryableFailure() throws {
        let server = try TestServer()
        defer { server.shutdown() }
        let link = try client(server, events: Inbox<LinkEvent>())
        #expect(
            link.transmit(peer: "server", bytes: [1])
                == TransportFailure(retryable: true, message: "no link to server"))
    }

    /// §mesh-18.3: a peer that answers pings keeps the link, and one that does not loses it.
    @Test func aLinkThatAnswersItsPingsStaysUp() throws {
        let server = try TestServer()
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        let link = try client(server, events: events, keepalive: .milliseconds(100))
        link.connect()
        #expect(events.next() == .ready(peer: "server"))
        #expect(events.next(within: 0.6) == nil, "a link that answers its pings was lost")
        link.close()
    }

    @Test func aPeerThatStopsAnsweringPingsLosesTheLink() throws {
        let server = try TestServer(answerPings: false)
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        let link = try client(server, events: events, keepalive: .milliseconds(100))
        link.connect()
        #expect(events.next() == .ready(peer: "server"))
        let lost = try #require(events.next(within: 3))
        guard case let .lost(_, reason) = lost else {
            throw Unexpected(description: "expected a lost link, got \(lost)")
        }
        #expect(reason.contains("pings"), "\(reason)")
    }

    @Test func aNameThatIsNotAPathSegmentCannotDial() throws {
        #expect(throws: WssClientError.notAPathSegment("a/b")) {
            try WssClient(base: "ws://127.0.0.1:1", own: "a/b", server: "server") { _ in }
        }
        #expect(throws: WssClientError.badBaseURL("http://127.0.0.1:1")) {
            try WssClient(base: "http://127.0.0.1:1", own: "client", server: "server") { _ in }
        }
    }
}
