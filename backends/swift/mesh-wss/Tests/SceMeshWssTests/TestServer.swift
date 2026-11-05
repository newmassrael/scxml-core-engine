// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A real WebSocket server on a loopback port, for the client's socket tests: the server machine's side
// of a link, as a peer writing and reading envelopes. It records what it is asked and read, and a test
// takes each in turn.

import Foundation
import NIOConcurrencyHelpers
import NIOCore
import NIOHTTP1
import NIOPosix
import NIOWebSocket

/// What a test waits on: a queue its producer pushes to from an event loop and the test takes from,
/// each take bounded in time so a missing event is a failure and not a hang.
final class Inbox<Element: Sendable>: @unchecked Sendable {
    private let condition = NSCondition()
    private var items: [Element] = []

    func push(_ element: Element) {
        condition.lock()
        items.append(element)
        condition.signal()
        condition.unlock()
    }

    /// The oldest item, or nil when none arrived within `seconds`.
    func next(within seconds: Double = 5) -> Element? {
        let deadline = Date().addingTimeInterval(seconds)
        condition.lock()
        defer { condition.unlock() }
        while items.isEmpty {
            if !condition.wait(until: deadline) {
                return nil
            }
        }
        return items.removeFirst()
    }

    /// Whether nothing is waiting.
    var isEmpty: Bool {
        condition.lock()
        defer { condition.unlock() }
        return items.isEmpty
    }
}

/// One accepted socket, from the server's side.
final class ServerSocket: @unchecked Sendable {
    let channel: any Channel

    init(channel: any Channel) {
        self.channel = channel
    }

    func send(binary bytes: [UInt8]) {
        var buffer = channel.allocator.buffer(capacity: bytes.count)
        buffer.writeBytes(bytes)
        channel.writeAndFlush(WebSocketFrame(fin: true, opcode: .binary, data: buffer), promise: nil)
    }

    func send(text: String) {
        var buffer = channel.allocator.buffer(capacity: text.utf8.count)
        buffer.writeString(text)
        channel.writeAndFlush(WebSocketFrame(fin: true, opcode: .text, data: buffer), promise: nil)
    }

    /// Close the way a server going away does, with status `code`.
    func close(code: WebSocketErrorCode = .normalClosure) {
        var buffer = channel.allocator.buffer(capacity: 2)
        buffer.write(webSocketErrorCode: code)
        let channel = channel
        channel.writeAndFlush(WebSocketFrame(fin: true, opcode: .connectionClose, data: buffer)).whenComplete { _ in
            channel.close(promise: nil)
        }
    }
}

final class TestServer: @unchecked Sendable {
    /// The request paths of every upgrade the server was asked for, in order.
    let paths = Inbox<String>()
    /// Every socket the server accepted.
    let sockets = Inbox<ServerSocket>()
    /// Every binary message a client wrote.
    let received = Inbox<[UInt8]>()
    /// The status of every close a client wrote.
    let closeCodes = Inbox<UInt16>()

    private let group: MultiThreadedEventLoopGroup
    private let listener: any Channel
    private let sockets_ = NIOLockedValueBox<[ServerSocket]>([])

    /// The port the server listens on.
    var port: Int { listener.localAddress?.port ?? 0 }

    /// - Parameters:
    ///   - refuseUpgrade: answer every upgrade request `404` instead of upgrading it.
    ///   - answerPings: answer a ping with a pong, as a WebSocket peer does.
    init(refuseUpgrade: Bool = false, answerPings: Bool = true) throws {
        let group = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        self.group = group
        let paths = paths
        let sockets = sockets
        let received = received
        let closeCodes = closeCodes
        let accepted = sockets_
        listener = try ServerBootstrap(group: group)
            .serverChannelOption(ChannelOptions.socketOption(.so_reuseaddr), value: 1)
            .childChannelInitializer { channel in
                let upgrader = NIOWebSocketServerUpgrader(
                    shouldUpgrade: { channel, head in
                        paths.push(head.uri)
                        return channel.eventLoop.makeSucceededFuture(refuseUpgrade ? nil : HTTPHeaders())
                    },
                    upgradePipelineHandler: { channel, _ in
                        // What answers a request that was not upgraded must not read the frames of
                        // one that was: it would be handed bytes it has no HTTP part for.
                        channel.pipeline.removeHandler(name: NotFound.name).flatMapThrowing {
                            try channel.pipeline.syncOperations.addHandler(
                                ServerFrames(received: received, closeCodes: closeCodes, answerPings: answerPings))
                            let socket = ServerSocket(channel: channel)
                            accepted.withLockedValue { $0.append(socket) }
                            sockets.push(socket)
                        }
                    })
                let configuration: NIOHTTPServerUpgradeConfiguration = (
                    upgraders: [upgrader], completionHandler: { _ in }
                )
                return channel.pipeline.configureHTTPServerPipeline(withServerUpgrade: configuration).flatMap {
                    channel.pipeline.addHandler(NotFound(), name: NotFound.name)
                }
            }
            .bind(host: "127.0.0.1", port: 0).wait()
    }

    /// Stop listening and close every socket a test left open.
    func shutdown() {
        for socket in sockets_.withLockedValue({ $0 }) {
            socket.channel.close(promise: nil)
        }
        try? listener.close().wait()
        try? group.syncShutdownGracefully()
    }
}

/// What the server reads from a client: binary messages recorded, a close answered and its status
/// kept, pings answered unless a test says the peer is deaf.
private final class ServerFrames: ChannelInboundHandler {
    typealias InboundIn = WebSocketFrame
    typealias OutboundOut = WebSocketFrame

    private let received: Inbox<[UInt8]>
    private let closeCodes: Inbox<UInt16>
    private let answerPings: Bool

    init(received: Inbox<[UInt8]>, closeCodes: Inbox<UInt16>, answerPings: Bool) {
        self.received = received
        self.closeCodes = closeCodes
        self.answerPings = answerPings
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        let frame = unwrapInboundIn(data)
        switch frame.opcode {
        case .binary:
            received.push(Array(frame.unmaskedData.readableBytesView))
        case .ping where answerPings:
            context.writeAndFlush(
                wrapOutboundOut(WebSocketFrame(fin: true, opcode: .pong, data: frame.unmaskedData)), promise: nil)
        case .connectionClose:
            var data = frame.unmaskedData
            if let code = data.readWebSocketErrorCode() {
                closeCodes.push(UInt16(webSocketErrorCode: code))
            }
            context.writeAndFlush(
                wrapOutboundOut(WebSocketFrame(fin: true, opcode: .connectionClose, data: frame.unmaskedData))
            ).whenComplete { _ in
                context.close(promise: nil)
            }
        default:
            break
        }
    }
}

/// A request that was not upgraded is answered `404`.
private final class NotFound: ChannelInboundHandler, RemovableChannelHandler {
    typealias InboundIn = HTTPServerRequestPart
    typealias OutboundOut = HTTPServerResponsePart

    static let name = "not-found"

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        guard case .end = unwrapInboundIn(data) else { return }
        var headers = HTTPHeaders()
        headers.add(name: "Content-Length", value: "0")
        headers.add(name: "Connection", value: "close")
        context.write(wrapOutboundOut(.head(HTTPResponseHead(version: .http1_1, status: .notFound, headers: headers))), promise: nil)
        context.writeAndFlush(wrapOutboundOut(.end(nil))).whenComplete { _ in
            context.close(promise: nil)
        }
    }
}
