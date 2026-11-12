// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// SCE_MESH.md §9.5: a Mesh request, `<invoke type="sce:mesh-rpc">`, of a `datamodel="sce-static"`
// machine is handed to the router the host registered, with its peer and its `<param>`s read from
// the machine's own fields when the invocation starts. The router answers through the engine's
// completion or failure call, and a machine that has none, or one with no binding for the peer,
// is told `error.execution` (W3C SCXML 6.4.1). The request is observed where the engine hands it to
// the router, so no transport is involved.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml

/// A router that keeps the starts it was handed, and answers each with `response`.
private final class Router {
    var starts: [HostInvokeRequest] = []
    var response: HostInvokeResponse?

    func handle(_ event: HostInvokeEvent) -> HostInvokeResponse? {
        if case .start(let request) = event {
            starts.append(request)
        }
        return response
    }
}

/// A machine whose Mesh requests go to `router`; none when `router` is nil.
private func requestingMachine(
    router: Router?
) -> (StatechartStaticMeshRequestPolicy, Engine<StatechartStaticMeshRequestPolicy>) {
    let policy = StatechartStaticMeshRequestPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    if let router {
        engine.registerMeshRpcInvoker(router.handle)
    }
    startRecordingARefusal(engine)
    return (policy, engine)
}

private func raise(_ engine: Engine<StatechartStaticMeshRequestPolicy>, _ name: String) {
    engine.raiseExternalByName(name, eventData: "")
    engine.step()
}

@Test func aRequestIsHandedToTheRouterWithItsPeerItsEnvelopeAndTheAuthorsPairs() {
    let router = Router()
    let (_, engine) = requestingMachine(router: router)
    raise(engine, "bump")
    raise(engine, "go")

    #expect(router.starts.count == 1)
    guard let request = router.starts.first else {
        return
    }
    #expect(request.processorType == meshRpcInvokeType)
    #expect(request.invokeID == "ask")
    #expect(request.src == "#motor")
    #expect(
        request.params == [
            "_mesh_event": ["service.request.force"], "_mesh_deadline_ms": ["250"],
            "force": ["8"], "speed": ["busy"],
        ]
    )
    // The author's pairs alone, typed as the data model holds them: the two envelope fields
    // are not in the event's data (SCE_MESH.md §9.5).
    #expect(request.eventData == "{\"force\":8,\"speed\":\"busy\"}")
}

@Test func theRequestIsReadWhenItStartsAndNotCopiedAtStartUp() {
    let router = Router()
    let (_, engine) = requestingMachine(router: router)
    raise(engine, "go")

    #expect(router.starts.count == 1)
    // Without `bump` the fields hold their initial values.
    #expect(router.starts.first?.params["force"] == ["6"])
    #expect(router.starts.first?.params["speed"] == ["idle"])
}

@Test func theExpressionThatNamesThePeerIsReadWhenTheRequestStarts() {
    let router = Router()
    let (_, engine) = requestingMachine(router: router)
    raise(engine, "ghost")
    raise(engine, "go")

    #expect(router.starts.first?.src == "#ghost")
}

@Test func anAnswerFromThePeerEndsTheRequestWithDoneInvoke() {
    let router = Router()
    let (policy, engine) = requestingMachine(router: router)
    raise(engine, "go")
    guard let request = router.starts.first else {
        Issue.record("the router was not asked")
        return
    }
    engine.completeHostInvoke(
        processorType: meshRpcInvokeType, invokeID: "ask", token: request.token, doneData: "{}",
        origin: EventOrigin(processor: meshRpcInvokeType, address: meshOriginScheme + "motor"))
    engine.step()

    #expect(policy.answered == 1)
    #expect(policy.failed == 0)
    #expect(policy.refused == 0)
}

@Test func aFailureEndsTheRequestWithErrorInvoke() {
    let router = Router()
    let (policy, engine) = requestingMachine(router: router)
    raise(engine, "go")
    guard let request = router.starts.first else {
        Issue.record("the router was not asked")
        return
    }
    engine.failHostInvoke(
        processorType: meshRpcInvokeType, invokeID: "ask", token: request.token, errorData: "")
    engine.step()

    #expect(policy.failed == 1)
    #expect(policy.answered == 0)
}

@Test func aMachineWithNoRouterIsToldErrorExecution() {
    let (policy, engine) = requestingMachine(router: nil)
    raise(engine, "go")

    #expect(policy.refused == 1)
    #expect(policy.answered == 0)
}

@Test func aRouterThatRefusesTheStartTellsTheMachineErrorExecution() {
    let router = Router()
    router.response = .refused("no binding for the peer")
    let (policy, engine) = requestingMachine(router: router)
    raise(engine, "go")

    #expect(router.starts.count == 1)
    #expect(policy.refused == 1)
    #expect(policy.answered == 0)
}
