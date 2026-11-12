// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// The Mesh router a parent gives the child session it starts (SCE_MESH.md, Child Sessions in
// Mesh): which router a child is given and when, how what arrives for it is applied before its
// step, how a request it makes ends, and what the router is told when the session ends.

/// A router that records what its child asked of it, and holds what it has for the child.
private final class RecordingRouter: ChildMeshRouter {
    var sends: [HostSendRequest] = []
    var invokes: [HostInvokeEvent] = []
    var pending: [MeshCall] = []
    var detached = 0
    var grandchild: RecordingRouter?
    var childrenAsked: [String] = []

    func send(_ request: HostSendRequest) -> [HostSendResponse] {
        sends.append(request)
        return []
    }

    func invoke(_ event: HostInvokeEvent) -> HostInvokeResponse? {
        invokes.append(event)
        return nil
    }

    func takeCalls() -> [MeshCall] {
        let taken = pending
        pending = []
        return taken
    }

    func detach() {
        detached += 1
    }

    func childRouter(invokeID: String, document: String) -> (any ChildMeshRouter)? {
        childrenAsked.append("\(invokeID):\(document)")
        return grandchild
    }

    /// The token of the one start of the Mesh request `invokeID` this router was asked for.
    func startToken(of invokeID: String) -> UInt64? {
        for event in invokes {
            if case .start(let request) = event, request.invokeID == invokeID {
                return request.token
            }
        }
        return nil
    }
}

private func makeChild() -> InvokedSession<ScriptedPolicy> {
    let policy = ScriptedPolicy()
    policy.transitions[.idle] = [
        FlatTransition(event: .go, target: .busy),
        FlatTransition(event: .doneInvokeProbe, target: .done),
        FlatTransition(event: .errorInvokeProbe, target: .busy),
        FlatTransition(event: .errorExecution, target: .busy),
    ]
    policy.transitions[.busy] = [FlatTransition(event: .again, target: .done)]
    return InvokedSession(policy: policy, sessionID: "parent.child", queue: ParentEventQueue())
}

private func makeParent(
    giving router: RecordingRouter?, asked: @escaping (String, String) -> Void = { _, _ in }
) throws -> Engine<ScriptedPolicy> {
    let (parent, _, _) = makeEngine { _ in }
    parent.registerChildMeshRouters { invokeID, document in
        asked(invokeID, document)
        return router
    }
    try parent.initialize()
    return parent
}

private let meshSend = HostSendRequest(processorType: meshProcessorType, eventName: "ping", target: "#peer")

@Test func aChildIsGivenTheRouterItsParentsFactoryAnsweredForTheInvokeAndTheDocument() throws {
    let router = RecordingRouter()
    var asked: [String] = []
    let parent = try makeParent(giving: router) { asked.append("\($0):\($1)") }
    let child = makeChild()
    parent.startChildMesh(of: child, invokeID: "worker", document: "worker_doc")
    try child.initialize()
    child.engine.performHostSend(meshSend)
    #expect(asked == ["worker:worker_doc"])
    #expect(router.sends == [meshSend])
}

@Test func aChildOfAParentWithNoFactoryHasNoRouterAndItsPeerSendIsAnError() throws {
    let (parent, _, _) = makeEngine { _ in }
    try parent.initialize()
    let child = makeChild()
    parent.startChildMesh(of: child, invokeID: "worker", document: "")
    try child.initialize()
    child.engine.performHostSend(meshSend)
    child.engine.tick()
    #expect(child.engine.getCurrentState() == .busy)
}

@Test func aFactoryThatAnswersNothingLeavesTheChildWithNoRouter() throws {
    let parent = try makeParent(giving: nil)
    let child = makeChild()
    parent.startChildMesh(of: child, invokeID: "worker", document: "")
    try child.initialize()
    child.engine.performHostSend(meshSend)
    child.engine.tick()
    #expect(child.engine.getCurrentState() == .busy)
}

@Test func theRouterOfAChildIsNotTheOneItsParentHost() throws {
    // A parent that has its own mesh router does not lend it to a child: the child's send goes
    // to the router the factory answered, and to nothing of the parent's.
    let parentRouter = RecordingRouter()
    let childRouter = RecordingRouter()
    let parent = try makeParent(giving: childRouter)
    parent.registerMeshRouter { request in parentRouter.send(request) }
    let child = makeChild()
    parent.startChildMesh(of: child, invokeID: "worker", document: "")
    try child.initialize()
    child.engine.performHostSend(meshSend)
    #expect(childRouter.sends == [meshSend])
    #expect(parentRouter.sends.isEmpty)
}

@Test func whatArrivedForAChildIsAppliedInTheOrderTheRouterReleasedItBeforeTheChildSteps() throws {
    // `go` then `again` takes the child through `busy` to `done`; the other order loses `again`
    // in `idle`, where nothing takes it.
    let ordered = RecordingRouter()
    let first = makeChild()
    try makeParent(giving: ordered).startChildMesh(of: first, invokeID: "w", document: "")
    try first.initialize()
    ordered.pending = [
        .raise(name: "go", metadata: meshEventMetadata(data: "", source: "peer", sendID: "s1")),
        .raise(name: "again", metadata: meshEventMetadata(data: "", source: "peer", sendID: "s2")),
    ]
    first.applyMeshCalls()
    first.step()
    #expect(first.engine.getCurrentState() == .done)
    #expect(ordered.pending.isEmpty)

    let reversed = RecordingRouter()
    let second = makeChild()
    try makeParent(giving: reversed).startChildMesh(of: second, invokeID: "w", document: "")
    try second.initialize()
    reversed.pending = [
        .raise(name: "again", metadata: meshEventMetadata(data: "", source: "peer", sendID: "s2")),
        .raise(name: "go", metadata: meshEventMetadata(data: "", source: "peer", sendID: "s1")),
    ]
    second.applyMeshCalls()
    second.step()
    #expect(second.engine.getCurrentState() == .busy)
}

@Test func aChildGivenNoRouterAppliesNothing() throws {
    let child = makeChild()
    try child.initialize()
    child.applyMeshCalls()
    child.step()
    #expect(child.engine.getCurrentState() == .idle)
}

@Test func anEventAPeerSentCarriesTheMeshOriginTheSendIdAndTheScxmlProcessor() {
    let metadata = meshEventMetadata(data: "{\"n\":1}", source: "sheet", sendID: "s9")
    #expect(metadata.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://sheet"))
    #expect(metadata.sender == nil, "the Mesh transport writes the pair, and no session sent it")
    #expect(metadata.sendID == "s9")
    #expect(metadata.data == "{\"n\":1}")
    #expect(metadata.eventType == .externalEvent)
}

@Test func aMeshRequestOfAChildReachesTheRouterAndEndsWithTheAnswerThatNamesThePeer() throws {
    let router = RecordingRouter()
    let child = makeChild()
    try makeParent(giving: router).startChildMesh(of: child, invokeID: "probe", document: "")
    try child.initialize()
    #expect(child.engine.performHostInvoke(HostInvokeRequest(processorType: meshRpcInvokeType, invokeID: "probe")))
    let token = router.startToken(of: "probe")
    #expect(token != nil)
    router.pending = [.endInvoke(invokeID: "probe", token: token ?? 0, failed: false, data: "{}", source: "peer")]
    child.applyMeshCalls()
    child.step()
    #expect(child.engine.getCurrentState() == .done)
}

@Test func aMeshRequestThatFailedEndsWithErrorInvoke() throws {
    let router = RecordingRouter()
    let child = makeChild()
    try makeParent(giving: router).startChildMesh(of: child, invokeID: "probe", document: "")
    try child.initialize()
    child.engine.performHostInvoke(HostInvokeRequest(processorType: meshRpcInvokeType, invokeID: "probe"))
    router.pending = [
        .endInvoke(invokeID: "probe", token: router.startToken(of: "probe") ?? 0, failed: true, data: "", source: nil)
    ]
    child.applyMeshCalls()
    child.step()
    #expect(child.engine.getCurrentState() == .busy)
}

@Test func anEndForAStartThatIsNoLongerRunningIsRefusedByTheTokenCheck() throws {
    let router = RecordingRouter()
    let child = makeChild()
    try makeParent(giving: router).startChildMesh(of: child, invokeID: "probe", document: "")
    try child.initialize()
    child.engine.performHostInvoke(HostInvokeRequest(processorType: meshRpcInvokeType, invokeID: "probe"))
    let stale = (router.startToken(of: "probe") ?? 0) + 100
    router.pending = [.endInvoke(invokeID: "probe", token: stale, failed: false, data: "{}", source: "peer")]
    child.applyMeshCalls()
    child.step()
    #expect(child.engine.getCurrentState() == .idle)
}

@Test func theRouterIsToldTheSessionEndedOnceAndNotAgain() throws {
    let router = RecordingRouter()
    let child = makeChild()
    try makeParent(giving: router).startChildMesh(of: child, invokeID: "w", document: "")
    try child.initialize()
    child.detachMesh()
    child.detachMesh()
    #expect(router.detached == 1)
}

@Test func aChildsOwnChildrenAreGivenTheRouterItsRouterAnswersFor() throws {
    let router = RecordingRouter()
    let grandRouter = RecordingRouter()
    router.grandchild = grandRouter
    let child = makeChild()
    try makeParent(giving: router).startChildMesh(of: child, invokeID: "w", document: "")
    try child.initialize()

    let grandchild = makeChild()
    child.engine.startChildMesh(of: grandchild, invokeID: "inner", document: "inner_doc")
    try grandchild.initialize()
    grandchild.engine.performHostSend(meshSend)
    #expect(router.childrenAsked == ["inner:inner_doc"])
    #expect(grandRouter.sends == [meshSend])
    #expect(router.sends.isEmpty)
}
