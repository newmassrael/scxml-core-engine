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
import SceLua
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
        engine.initialize()
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
                errorData: "\"unreachable\"", origin: "mesh://motor"))
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
