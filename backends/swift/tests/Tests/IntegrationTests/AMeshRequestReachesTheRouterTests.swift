// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19: an `<invoke type="sce:mesh-rpc">` reaches the host's Mesh router through the
// runtime's mesh-rpc door. Swift AOT path, over an `ecmascript` document: the engine reads the request when the
// invocation starts, and the router is handed it.
//
// Fixture: integration_resources/a_mesh_request_reaches_the_router/a_mesh_request_reaches_the_router.scxml
// (canonical, shared with the other channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceForgeRuntime
import SceLua
import SceMesh
import SceRuntime
import Testing

/// What the router was started with, in order.
private final class Requests {
    var started: [HostInvokeRequest] = []
}

@Suite struct AMeshRequestReachesTheRouter {
    /// Runs the machine into `asking`, with a router recording into `requests` when given. The router keeps every
    /// request and answers none, so the test decides how each one ends.
    private func started(
        _ requests: Requests?
    ) -> (AMeshRequestReachesTheRouterPolicy, Engine<AMeshRequestReachesTheRouterPolicy>) {
        let policy = AMeshRequestReachesTheRouterPolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        if let requests {
            engine.registerMeshRpcInvoker { event in
                if case .start(let request) = event { requests.started.append(request) }
                return nil
            }
        }
        startRecordingARefusal(engine)
        engine.tick()
        return (policy, engine)
    }

    /// The one request the router was started with, checked against what the document wrote.
    private func theRequest(_ requests: Requests) throws -> HostInvokeRequest {
        #expect(requests.started.count == 1, "exactly one request reaches the router: \(requests.started)")
        let request = try #require(requests.started.first)
        #expect(request.processorType == meshRpcInvokeType)
        #expect(request.invokeID == "ask")
        #expect(request.src == "#motor")
        #expect(
            request.params == [
                "_mesh_event": ["service.request.force"],
                "_mesh_deadline_ms": ["250"],
                "force": ["3"],
                "speed": ["3"],
            ])
        // The author's pairs alone, typed: `force` was computed, `speed` was written as a string, and the
        // envelope fields are not payload.
        #expect(request.eventData == #"{"force":3,"speed":"3"}"#)
        return request
    }

    private func expectEnded(
        _ policy: AMeshRequestReachesTheRouterPolicy, _ engine: Engine<AMeshRequestReachesTheRouterPolicy>,
        answered: Int64, failed: Int64, refused: Int64
    ) {
        let wrong = mismatches([("answered", answered), ("failed", failed), ("refused", refused)]) {
            policy.readScriptValue($0)
        }
        #expect(wrong.isEmpty, "observed: \(wrong)")
        #expect(engine.terminalState == .done)
    }

    /// The router answers: done.invoke.ask ends the run.
    @Test func aMeshRequestIsAnsweredThroughTheRouter() throws {
        let requests = Requests()
        let (policy, engine) = started(requests)
        let request = try theRequest(requests)
        #expect(
            engine.completeHostInvoke(
                processorType: meshRpcInvokeType, invokeID: "ask", token: request.token, doneData: "\"ok\""))
        engine.tick()
        expectEnded(policy, engine, answered: 1, failed: 0, refused: 0)
    }

    /// The router fails the request: error.invoke.ask, carrying the router's data.
    @Test func aMeshRequestTheRouterFailsIsErrorInvoke() throws {
        let requests = Requests()
        let (policy, engine) = started(requests)
        let request = try theRequest(requests)
        #expect(
            engine.failHostInvoke(
                processorType: meshRpcInvokeType, invokeID: "ask", token: request.token,
                errorData: "\"unreachable\"",
                origin: EventOrigin(processor: meshRpcInvokeType, address: "mesh://motor")))
        engine.tick()
        expectEnded(policy, engine, answered: 0, failed: 1, refused: 0)
    }

    /// No router was registered: the invoke names a type nobody runs, which is error.execution (W3C SCXML 6.4.1).
    @Test func aMeshRequestWithNoRouterIsTheErrorAnInvokeNobodyRunsIs() {
        let (policy, engine) = started(nil)
        engine.tick()
        expectEnded(policy, engine, answered: 0, failed: 0, refused: 1)
    }
}

// The same document through the Swift host core itself (`SceMesh`), and not only through the door it is
// registered at.

/// A clock that stands still and ids that count.
private final class Fixed: Environment {
    private var ids = 0

    func nowMs() -> Int64 { 0 }

    func nowUnixMs() -> Int64 { 1_000_000 }

    func envelopeID() -> [UInt8] {
        ids += 1
        return [UInt8](repeating: UInt8(truncatingIfNeeded: ids), count: 16)
    }

    func jitterDraw() -> Int64 { 0 }
}

/// A transport that keeps what it was handed.
private final class Wire: Transport {
    var sent: [(peer: String, bytes: [UInt8])] = []

    func transmit(peer: String, bytes: [UInt8]) -> TransportFailure? {
        sent.append((peer, bytes))
        return nil
    }
}

@Suite struct AMeshRequestReachesTheRouterThroughTheHostCore {
    /// The requester's endpoint, bound to `motor` when `bound`, sending into `wire`.
    private func endpoint(bound: Bool, wire: Wire) -> Endpoint {
        let router = Router(machine: "brake", dedupWindow: 8, gapTimeoutMs: 50)
        if bound {
            router.addPeer(
                "motor",
                config: PeerConfig(
                    transport: "wss",
                    buffer: nil,
                    retry: nil,
                    stampSequence: false,
                    delivery: Delivery(dedup: true, ordered: false),
                    responders: ["motor"],
                    deadlineMs: nil,
                    replyEvents: []
                ))
        }
        return Endpoint(router: router, transport: wire, environment: Fixed())
    }

    /// Runs the machine into `asking` with `endpoint` serving its Mesh traffic.
    private func startedThrough(
        _ endpoint: Endpoint
    ) -> (AMeshRequestReachesTheRouterPolicy, Engine<AMeshRequestReachesTheRouterPolicy>) {
        let policy = AMeshRequestReachesTheRouterPolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        register(endpoint, on: engine)
        startRecordingARefusal(engine)
        engine.tick()
        return (policy, engine)
    }

    private func expectEnded(
        _ policy: AMeshRequestReachesTheRouterPolicy, _ engine: Engine<AMeshRequestReachesTheRouterPolicy>,
        answered: Int64, failed: Int64, refused: Int64
    ) {
        let wrong = mismatches([("answered", answered), ("failed", failed), ("refused", refused)]) {
            policy.readScriptValue($0)
        }
        #expect(wrong.isEmpty, "observed: \(wrong)")
        #expect(engine.terminalState == .done)
    }

    /// The request reaches `motor` as an `RpcRequest`, and motor's `ok` reply ends the invoke with
    /// `done.invoke.ask`.
    @Test func aReplyThroughTheHostCoreAnswersTheDocument() throws {
        let wire = Wire()
        let endpoint = endpoint(bound: true, wire: wire)
        let (policy, engine) = startedThrough(endpoint)

        #expect(wire.sent.count == 1, "exactly one envelope reaches the wire")
        let sent = try #require(wire.sent.first)
        #expect(sent.peer == "motor")
        var cursor = SceCursor(sent.bytes)
        let request = try Envelope.decode(&cursor)
        #expect(request.pattern == .rpcRequest)
        #expect(request.event_type == "service.request.force")
        #expect(String(decoding: request.data, as: UTF8.self) == #"{"force":3,"speed":"3"}"#)
        // The document's own deadline, on the wall clock.
        #expect(request.deadline_unix_ms == 1_000_250)

        var reply = Envelope()
        reply.id = [UInt8](repeating: 0xAA, count: 16)
        reply.source = "motor"
        reply.event_type = "service.response.force"
        reply.pattern = .rpcReply
        reply.datacontenttype = .json
        reply.data = Array(#""ok""#.utf8)
        reply.invoke_id = request.invoke_id
        reply.rpc_status = .ok
        let bytes = try #require(encodeToBytes({ sink in try reply.encode(into: &sink) }))
        endpoint.receive(peer: "motor", bytes: bytes)
        apply(endpoint.takeCalls(), to: engine)
        engine.tick()
        expectEnded(policy, engine, answered: 1, failed: 0, refused: 0)
    }

    /// A target the host core has no binding for cannot reach the wire: the invocation is refused, and
    /// the document sees `error.execution` (SCE_MESH.md §mesh-9.5's pre-envelope tier).
    @Test func anUnboundTargetThroughTheHostCoreIsErrorExecution() {
        let wire = Wire()
        let (policy, engine) = startedThrough(endpoint(bound: false, wire: wire))
        #expect(wire.sent.isEmpty, "nothing reached the wire")
        expectEnded(policy, engine, answered: 0, failed: 0, refused: 1)
    }
}
