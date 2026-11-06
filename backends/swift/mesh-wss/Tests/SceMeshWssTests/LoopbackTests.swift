// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-18 mesh-18.2 mesh-18.4 mesh-19
//
// The Swift core against the Rust core over one real WebSocket (SCE_MESH.md §mesh-18). The Rust side
// is backends/rust/mesh/examples/wss_peer.rs, a server machine that answers each `ping` with a `pong`
// carrying the same `_event.data`; the lane that runs this builds it and names its path in
// `SCE_MESH_WSS_PEER`.
//
// What it proves is what the cores' shared literal tests cannot: the Rust core decodes an envelope
// the Swift core encoded, and the Swift core decodes one the Rust core encoded, on the wire the
// binding fixes (one envelope per binary message, §mesh-18.2). That pairs the Rust core's
// tokio-tungstenite server with the Swift core's SwiftNIO client. Both sides build their routers from
// the peer tables the build generated from one deployment (tests/mesh/wss_loopback/deploy.yaml,
// SCE_MESH.md §mesh-19), so the configuration they agree on is the one deploy.yaml says.

import Foundation
import NIOCore
import SceMesh
import SceMeshWss
import SceRuntime
import Testing

private final class Wall: Environment {
    private var now: Int64 = 0
    private var ids = 0

    func nowMs() -> Int64 {
        now += 1
        return now
    }

    func nowUnixMs() -> Int64 { Int64(Date().timeIntervalSince1970 * 1000) }

    func envelopeID() -> [UInt8] {
        ids += 1
        return [UInt8](repeating: UInt8(truncatingIfNeeded: 0x30 + ids), count: 16)
    }

    func jitterDraw() -> Int64 { 0 }
}

/// The lines the peer writes on its standard output, read as they come.
private final class Lines {
    private let handle: FileHandle
    private var pending = Data()

    init(_ handle: FileHandle) {
        self.handle = handle
    }

    /// The next line, or nil when the peer closed its output.
    func next() -> String? {
        while true {
            if let newline = pending.firstIndex(of: 0x0A) {
                let line = String(decoding: pending[pending.startIndex..<newline], as: UTF8.self)
                pending.removeSubrange(pending.startIndex...newline)
                return line
            }
            let chunk = handle.availableData
            if chunk.isEmpty {
                if pending.isEmpty { return nil }
                defer { pending.removeAll() }
                return String(decoding: pending, as: UTF8.self)
            }
            pending.append(chunk)
        }
    }
}

@Suite(.serialized) struct LoopbackTests {
    private func firstLine(_ lines: Lines) throws -> String {
        guard let line = lines.next() else {
            throw Unexpected(description: "the Rust peer exited before it listened")
        }
        return line
    }

    @Test func theSwiftAndRustCoresReadEachOthersEnvelopes() throws {
        guard let path = ProcessInfo.processInfo.environment["SCE_MESH_WSS_PEER"], !path.isEmpty else {
            throw Unexpected(description: "the lane did not name the Rust peer (SCE_MESH_WSS_PEER)")
        }
        #expect(
            FileManager.default.isExecutableFile(atPath: path),
            "the Rust peer is not built at \(path): `cargo build -p sce-rust-mesh --features wss --example wss_peer`")

        let peer = Process()
        peer.executableURL = URL(fileURLWithPath: path)
        let output = Pipe()
        peer.standardOutput = output
        try peer.run()
        defer {
            if peer.isRunning { peer.terminate() }
        }
        let lines = Lines(output.fileHandleForReading)
        let listening = try firstLine(lines)
        guard listening.hasPrefix("LISTENING "), let port = Int(listening.dropFirst("LISTENING ".count)) else {
            throw Unexpected(description: "unexpected first line: \(listening)")
        }

        // The machine as tests/mesh/wss_loopback/deploy.yaml binds it, from the table the build
        // generated. The server's port is the one thing the deployment cannot know: the Rust peer
        // takes an ephemeral one.
        let machine = ClientMeshPeers.machine
        let server = try #require(machine.peer("server"), "the deployment binds no #server")
        guard case let .wssDial(url, keepaliveMs) = server.link else {
            throw Unexpected(description: "the deployment does not dial #server: \(server.link)")
        }
        let base = String(url[..<(url.lastIndex(of: ":") ?? url.endIndex)]) + ":\(port)"
        var configuration = WssConfiguration()
        if let keepaliveMs {
            configuration.keepalive = .milliseconds(Int64(keepaliveMs))
        }

        let events = Inbox<LinkEvent>()
        let link = try WssClient(base: base, own: machine.name, server: server.name, configuration: configuration) {
            events.push($0)
        }
        let endpoint = Endpoint(router: machine.router(), transport: link, environment: Wall())
        link.connect()
        let ready = try #require(events.next(within: 10))
        #expect(ready == .ready(peer: "server"))
        deliver(ready, to: endpoint)

        // Text the escaping rules would get wrong if either side had its own: a quote, a backslash, a
        // C0 control as JSON writes it, and a two- and a three-byte UTF-8 character (U+00E9, U+20AC).
        let data = #"{"n":1,"s":"q\"\\ \u0001 é €"}"#
        endpoint.send(
            HostSendRequest(
                processorType: meshProcessorType, eventName: "ping", target: "#server", sendID: "ask-1",
                eventData: data))
        #expect(endpoint.takeHostErrors().isEmpty)

        let reply = try #require(events.next(within: 10))
        guard case .received = reply else {
            throw Unexpected(description: "expected a received message, got \(reply)")
        }
        deliver(reply, to: endpoint)
        let calls = endpoint.takeCalls()
        guard calls.count == 1, case let .raise(pong) = calls[0] else {
            throw Unexpected(description: "expected one event, got \(calls)")
        }
        #expect(pong.name == "pong")
        #expect(pong.metadata.data == data)
        #expect(pong.metadata.sendID == "reply-1")
        #expect(pong.metadata.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://server"))
        #expect(pong.metadata.sender == nil, "no session sent it")

        // §mesh-9.5, §mesh-10.7: a request and its reply. The Swift core writes the RpcRequest; the
        // Rust core delivers it with its wire invokeid, and stamps the reply the server sends while
        // handling it; the Swift core matches that reply to the invocation.
        let started = endpoint.invoke(
            HostInvokeRequest(
                processorType: meshRpcInvokeType, invokeID: "echo", src: "#server",
                params: [meshEventParam: ["service.request.echo"]], eventData: #"{"x":1}"#, token: 1))
        #expect(started.refusal == nil, "the request was refused: \(String(describing: started.refusal))")
        #expect(endpoint.takeHostErrors().isEmpty)

        let answer = try #require(events.next(within: 10))
        guard case .received = answer else {
            throw Unexpected(description: "expected a received message, got \(answer)")
        }
        deliver(answer, to: endpoint)
        let ended = endpoint.takeCalls()
        guard ended.count == 1,
            case let .endInvoke(invokeID, token, failed, endData, source) = ended[0]
        else {
            throw Unexpected(description: "expected the request to end, got \(ended)")
        }
        #expect(invokeID == "echo")
        #expect(token == 1)
        #expect(!failed)
        #expect(endData == #"{"x":1}"#)
        #expect(source == "server")

        link.close()
        peer.waitUntilExit()
        #expect(lines.next() == "DONE 2")
        #expect(peer.terminationStatus == 0)
    }
}
