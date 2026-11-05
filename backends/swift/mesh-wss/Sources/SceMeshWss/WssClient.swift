// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The WebSocket binding's client (SCE_MESH.md §mesh-18) over SwiftNIO: the socket plumbing between
// one server link and the router core, and nothing else. It dials `<base>/sce-mesh/1/<own>`, reports
// what happens on the link as `LinkEvent`s, and is the `Transport` the core's sends reach the socket
// through. It is the Swift row of the implementations table (§mesh-18.4), beside the Rust client over
// tokio-tungstenite and the Kotlin one over OkHttp.
//
// NIO calls back on the link's event loop, so `events` runs there; the host posts each event to the
// thread that steps its engine and calls `deliver(_:to:)` there, which keeps the endpoint confined to
// that thread.

import Foundation
import NIOConcurrencyHelpers
import NIOCore
import NIOHTTP1
import NIOPosix
import NIOSSL
import NIOWebSocket
import SceMesh

/// Why a client could not be made.
public enum WssClientError: Error, Equatable, Sendable {
    /// `base` is not a `ws://` or `wss://` URL with a host.
    case badBaseURL(String)

    /// The client's own machine name is not a path segment as it stands (§mesh-18.1).
    case notAPathSegment(String)
}

/// What a link is given beyond where to dial.
public struct WssConfiguration: Sendable {
    /// The §mesh-18.3 ping interval: the link pings on it and is lost when a pong does not come back
    /// within the next one. The binding's 30 s when the deployment sets none.
    public var keepalive: TimeAmount = .seconds(30)

    /// Headers added to the upgrade request, such as the authentication header §mesh-18.1 leaves to
    /// the deployment.
    public var headers: HTTPHeaders = HTTPHeaders()

    /// The TLS settings of a `wss://` link. By default the platform's trusted roots, and the
    /// certificate checked against the host the URL names.
    public var tls: TLSConfiguration = .makeClientConfiguration()

    /// The largest message the link accepts, in bytes: an envelope bigger than it is a protocol
    /// error, as a frame bigger than the socket's own bound is.
    public var maxMessageBytes: Int = 16 * 1024 * 1024

    /// How long a dial may take before it is a lost link.
    public var connectTimeout: TimeAmount = .seconds(10)

    public init() {}
}

/// One link to the server machine `server` at `base` (a `ws://` or `wss://` URL before the binding's
/// path), as the machine `own`.
///
/// Thread-safe: `connect`, `close` and `transmit` may be called from any thread; what the link
/// observes is reported through `events` on its event loop.
public final class WssClient: Transport, @unchecked Sendable {
    /// What one dial of the link is, guarded by `lock`. A callback of an older dial finds the
    /// generation moved on and does nothing.
    private struct State {
        var generation = 0
        /// The dial `generation` names has neither ended nor been closed.
        var active = false
        var channel: (any Channel)?
        /// The upgrade completed.
        var open = false
        /// A ping was sent and its pong has not come back.
        var awaitingPong = false
        var keepalive: RepeatedTask?
        var nextSequence: UInt64 = 0
        /// What was handed to the socket and not yet written, oldest first.
        var handed: [(sequence: UInt64, bytes: [UInt8])] = []
    }

    private let host: String
    private let port: Int
    private let secure: Bool
    private let path: String
    private let hostHeader: String
    private let server: String
    private let configuration: WssConfiguration
    private let group: any EventLoopGroup
    private let events: @Sendable (LinkEvent) -> Void
    private let state = NIOLockedValueBox(State())

    /// - Parameters:
    ///   - base: the URL before the binding's path, `ws://host[:port][/base]` or `wss://…`.
    ///   - own: this machine's name, the last segment of the path the client opens.
    ///   - server: the peer's name, which the events name.
    ///   - group: where the link runs. The process-wide singleton group when omitted.
    ///   - events: told what happens on the link, on its event loop.
    public init(
        base: String, own: String, server: String, configuration: WssConfiguration = WssConfiguration(),
        group: any EventLoopGroup = MultiThreadedEventLoopGroup.singleton,
        events: @escaping @Sendable (LinkEvent) -> Void
    ) throws {
        guard isPathSegment(own) else {
            throw WssClientError.notAPathSegment(own)
        }
        guard let components = URLComponents(string: base), let scheme = components.scheme?.lowercased(),
            scheme == "ws" || scheme == "wss", let host = components.host, !host.isEmpty
        else {
            throw WssClientError.badBaseURL(base)
        }
        self.secure = scheme == "wss"
        self.host = host
        self.port = components.port ?? (secure ? 443 : 80)
        var prefix = components.path
        while prefix.hasSuffix("/") {
            prefix.removeLast()
        }
        self.path = prefix + wssPathPrefix + own
        self.hostHeader = components.port.map { "\(host):\($0)" } ?? host
        self.server = server
        self.configuration = configuration
        self.group = group
        self.events = events
    }

    /// Dial the server. A client reconnects by calling this again, on its own backoff (§mesh-18.3).
    public func connect() {
        let generation = state.withLockedValue { state -> Int in
            state.generation += 1
            state.active = true
            state.open = false
            state.awaitingPong = false
            state.channel = nil
            state.handed.removeAll()
            return state.generation
        }
        let bootstrap = ClientBootstrap(group: group)
            .connectTimeout(configuration.connectTimeout)
            .channelInitializer { channel in
                channel.eventLoop.makeCompletedFuture(withResultOf: {
                    try self.install(on: channel, generation: generation)
                })
            }
        bootstrap.connect(host: host, port: port).whenFailure { error in
            self.lost(generation, "the dial failed: \(error)")
        }
    }

    /// Close the link as a client going away; its loss is reported as any other.
    public func close() {
        let channel = state.withLockedValue { $0.channel }
        guard let channel else { return }
        var reason = channel.allocator.buffer(capacity: 32)
        reason.write(webSocketErrorCode: .goingAway)
        reason.writeString("the client is going away")
        let frame = WebSocketFrame(fin: true, opcode: .connectionClose, maskKey: .random(), data: reason)
        channel.writeAndFlush(frame).whenComplete { _ in
            channel.close(promise: nil)
        }
    }

    public func transmit(peer: String, bytes: [UInt8]) -> TransportFailure? {
        let handed = state.withLockedValue { state -> (channel: any Channel, sequence: UInt64, generation: Int)? in
            guard state.active, state.open, let channel = state.channel else {
                return nil
            }
            let sequence = state.nextSequence
            state.nextSequence += 1
            state.handed.append((sequence, bytes))
            return (channel, sequence, state.generation)
        }
        guard let handed else {
            return TransportFailure(retryable: true, message: "no link to \(peer)")
        }
        var buffer = handed.channel.allocator.buffer(capacity: bytes.count)
        buffer.writeBytes(bytes)
        let frame = WebSocketFrame(fin: true, opcode: .binary, maskKey: .random(), data: buffer)
        handed.channel.writeAndFlush(frame).whenSuccess {
            self.written(generation: handed.generation, sequence: handed.sequence)
        }
        return nil
    }

    /// The socket has written `sequence`: it is no longer something a closing link leaves unsent.
    private func written(generation: Int, sequence: UInt64) {
        state.withLockedValue { state in
            guard state.generation == generation else { return }
            state.handed.removeAll { $0.sequence == sequence }
        }
    }

    /// The link ended, or never opened (the upgrade was refused, the dial failed): report what it
    /// left unwritten, then the loss, once per dial.
    private func lost(_ generation: Int, _ reason: String) {
        let ended = state.withLockedValue { state -> (unsent: [[UInt8]], channel: (any Channel)?)? in
            guard state.generation == generation, state.active else {
                return nil
            }
            state.active = false
            state.open = false
            let unsent = state.handed.map(\.bytes)
            state.handed.removeAll()
            let channel = state.channel
            state.channel = nil
            state.keepalive?.cancel()
            state.keepalive = nil
            return (unsent, channel)
        }
        guard let ended else { return }
        ended.channel?.close(promise: nil)
        for bytes in ended.unsent {
            events(.unsent(peer: server, bytes: bytes))
        }
        events(.lost(peer: server, reason: reason))
    }

    // MARK: the pipeline

    /// The handlers of one dial: TLS when the URL says `wss`, the HTTP/1.1 client with the WebSocket
    /// upgrade, and what reads the upgrade's refusal. The frame handler joins when the upgrade
    /// completes.
    private func install(on channel: any Channel, generation: Int) throws {
        let sync = channel.pipeline.syncOperations
        if secure {
            let context = try NIOSSLContext(configuration: configuration.tls)
            try sync.addHandler(try NIOSSLClientHandler(context: context, serverHostname: host))
        }
        let upgrader = NIOWebSocketClientUpgrader(
            requestKey: NIOWebSocketClientUpgrader.randomRequestKey(),
            maxFrameSize: configuration.maxMessageBytes
        ) { channel, _ in
            channel.eventLoop.makeCompletedFuture(withResultOf: {
                try self.opened(on: channel, generation: generation)
            })
        }
        let request = UpgradeRequest(
            path: path, host: hostHeader, headers: configuration.headers,
            refused: { status in self.lost(generation, "the upgrade was answered \(status.code)") },
            failed: { reason in self.lost(generation, reason) })
        let upgrade: NIOHTTPClientUpgradeConfiguration = (
            upgraders: [upgrader],
            completionHandler: { context in
                context.channel.pipeline.removeHandler(request, promise: nil)
            }
        )
        try sync.addHTTPClientHandlers(withClientUpgrade: upgrade)
        try sync.addHandler(request)
    }

    /// The upgrade completed: the link is ready (§mesh-18.3).
    private func opened(on channel: any Channel, generation: Int) throws {
        let current = state.withLockedValue { state -> Bool in
            guard state.generation == generation, state.active else {
                return false
            }
            state.channel = channel
            state.open = true
            state.awaitingPong = false
            return true
        }
        guard current else {
            // A dial the host has since replaced or closed: nothing to report, nothing to keep.
            channel.close(promise: nil)
            return
        }
        let sync = channel.pipeline.syncOperations
        try sync.addHandler(
            NIOWebSocketFrameAggregator(
                minNonFinalFragmentSize: 0, maxAccumulatedFrameCount: 64,
                maxAccumulatedFrameSize: configuration.maxMessageBytes))
        try sync.addHandler(Frames(client: self, generation: generation))
        let task = channel.eventLoop.scheduleRepeatedTask(
            initialDelay: configuration.keepalive, delay: configuration.keepalive
        ) { _ in
            self.ping(channel, generation: generation)
        }
        state.withLockedValue { state in
            if state.generation == generation, state.active {
                state.keepalive = task
            } else {
                task.cancel()
            }
        }
        events(.ready(peer: server))
    }

    /// One keepalive turn (§mesh-18.3): the link is lost when the last ping was never answered,
    /// and pinged again otherwise.
    private func ping(_ channel: any Channel, generation: Int) {
        let unanswered = state.withLockedValue { state -> Bool? in
            guard state.generation == generation, state.active else {
                return nil
            }
            if state.awaitingPong {
                return true
            }
            state.awaitingPong = true
            return false
        }
        switch unanswered {
        case nil:
            return
        case true?:
            lost(generation, "the peer stopped answering pings")
        case false?:
            let frame = WebSocketFrame(fin: true, opcode: .ping, maskKey: .random(), data: channel.allocator.buffer(capacity: 0))
            channel.writeAndFlush(frame, promise: nil)
        }
    }

    fileprivate func pongReceived(_ generation: Int) {
        state.withLockedValue { state in
            if state.generation == generation {
                state.awaitingPong = false
            }
        }
    }

    fileprivate func received(_ generation: Int, _ bytes: [UInt8]) {
        let open = state.withLockedValue { $0.generation == generation && $0.active && $0.open }
        if open {
            events(.received(peer: server, bytes: bytes))
        }
    }

    fileprivate func ended(_ generation: Int, _ reason: String) {
        lost(generation, reason)
    }
}

/// The upgrade request: `GET <path>` with the headers the upgrader and the host add, and the report
/// of an answer that is not `101`.
private final class UpgradeRequest: ChannelInboundHandler, RemovableChannelHandler, @unchecked Sendable {
    // Confined to its channel's event loop: it holds only values fixed at construction and is
    // named by the upgrade's completion handler, which removes it from the pipeline.
    typealias InboundIn = HTTPClientResponsePart
    typealias OutboundOut = HTTPClientRequestPart

    private let path: String
    private let host: String
    private let headers: HTTPHeaders
    private let refused: @Sendable (HTTPResponseStatus) -> Void
    private let failed: @Sendable (String) -> Void

    init(
        path: String, host: String, headers: HTTPHeaders, refused: @escaping @Sendable (HTTPResponseStatus) -> Void,
        failed: @escaping @Sendable (String) -> Void
    ) {
        self.path = path
        self.host = host
        self.headers = headers
        self.refused = refused
        self.failed = failed
    }

    func channelActive(context: ChannelHandlerContext) {
        var head = HTTPRequestHead(version: .http1_1, method: .GET, uri: path)
        var all = HTTPHeaders()
        all.add(name: "Host", value: host)
        all.add(name: "Content-Length", value: "0")
        for (name, value) in headers {
            all.add(name: name, value: value)
        }
        head.headers = all
        context.write(wrapOutboundOut(.head(head)), promise: nil)
        context.writeAndFlush(wrapOutboundOut(.end(nil)), promise: nil)
        context.fireChannelActive()
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        if case let .head(response) = unwrapInboundIn(data), response.status != .switchingProtocols {
            refused(response.status)
            context.close(promise: nil)
        }
    }

    /// What goes wrong before the upgrade completes (a TLS handshake the client refuses, a peer that
    /// hangs up) is a link that never opened: it is reported, since nothing else on the pipeline yet
    /// can, and a host that was told nothing would wait on it for ever.
    func errorCaught(context: ChannelHandlerContext, error: any Error) {
        failed("\(error)")
        context.close(promise: nil)
    }

    func channelInactive(context: ChannelHandlerContext) {
        failed("the connection closed before the upgrade completed")
        context.fireChannelInactive()
    }
}

/// What the link reads once it is a WebSocket: one envelope per binary message (§mesh-18.2), text
/// refused with the 1003 close, pings answered, and the peer's close answered.
private final class Frames: ChannelInboundHandler {
    typealias InboundIn = WebSocketFrame
    typealias OutboundOut = WebSocketFrame

    private let client: WssClient
    private let generation: Int

    init(client: WssClient, generation: Int) {
        self.client = client
        self.generation = generation
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        let frame = unwrapInboundIn(data)
        switch frame.opcode {
        case .binary:
            client.received(generation, Array(frame.unmaskedData.readableBytesView))
        case .text:
            // §mesh-18.2: text is not an envelope.
            close(context, code: .unacceptableData, reason: "an envelope is a binary message")
            client.ended(generation, "the peer sent a text message")
        case .ping:
            let pong = WebSocketFrame(fin: true, opcode: .pong, maskKey: .random(), data: frame.unmaskedData)
            context.writeAndFlush(wrapOutboundOut(pong), promise: nil)
        case .pong:
            client.pongReceived(generation)
        case .connectionClose:
            var data = frame.unmaskedData
            let code = data.readWebSocketErrorCode().map { String(UInt16(webSocketErrorCode: $0)) } ?? "no status"
            close(context, code: .normalClosure, reason: nil)
            client.ended(generation, "the peer closed the link (\(code))")
        default:
            break
        }
    }

    func channelInactive(context: ChannelHandlerContext) {
        client.ended(generation, "the link closed")
        context.fireChannelInactive()
    }

    func errorCaught(context: ChannelHandlerContext, error: any Error) {
        client.ended(generation, "\(error)")
        context.close(promise: nil)
    }

    private func close(_ context: ChannelHandlerContext, code: WebSocketErrorCode, reason: String?) {
        var buffer = context.channel.allocator.buffer(capacity: 2 + (reason?.utf8.count ?? 0))
        buffer.write(webSocketErrorCode: code)
        if let reason {
            buffer.writeString(reason)
        }
        let frame = WebSocketFrame(fin: true, opcode: .connectionClose, maskKey: .random(), data: buffer)
        context.writeAndFlush(wrapOutboundOut(frame)).whenComplete { _ in
            context.close(promise: nil)
        }
    }
}
