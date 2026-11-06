// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Mesh router a child session is given by its parent (SCE_MESH.md, Child Sessions in Mesh;
// docs/adr/0043). The Swift port of Kotlin's `ChildMeshRouter`, `MeshCall` and
// `meshEventMetadata`, and of the engine's `registerChildMeshRouters` / `applyMeshCalls` /
// `detachMesh`.

/// `_event.origin` of a Mesh-delivered event is this scheme and the sending machine's name
/// (SCE_MESH.md §10.7).
public let meshOriginScheme = "mesh://"

/// The type of the Mesh router's `<invoke>` (SCE_MESH.md §9.5): the build lowers a request to a
/// peer to a host-run invoke of this type, and the host serves it with the invoker it registered.
public let meshRpcInvokeType = "sce:mesh-rpc"

/// The `_event` of an event that arrived over Mesh from `source`, as SCE_MESH.md §10.7 tabulates
/// it: external, origin `mesh://<source>`, the SCXML processor as origin type, and the send id the
/// envelope carried.
///
/// The one place it is built: a host's endpoint builds what it hands a top-level machine with it,
/// and what it hands a child session, so a child cannot tell a peer's event from the one its
/// parent would have been given. The Mesh transport writes the pair, and no session is its sender
/// (docs/adr/0090, section 2).
public func meshEventMetadata(data: String, source: String, sendID: String) -> EventMetadata {
    EventMetadata(
        data: data,
        eventType: .externalEvent,
        sendID: sendID,
        origin: EventOrigin(processor: scxmlEventProcessorType, address: meshOriginScheme + source)
    )
}

/// One thing a Mesh endpoint has for the engine it serves, in the order the endpoint came to know
/// it: what `Engine.applyMeshCall` applies.
public enum MeshCall {
    /// Raise `name` on the external queue with the `_event` the endpoint built for it.
    case raise(name: String, metadata: EventMetadata)

    /// End the start `token` of the Mesh request `invokeID`, with `done.invoke`, or with
    /// `error.invoke` when `failed`, carrying `data` as `_event.data`. `source` is the machine
    /// that answered, nil for a deadline the requester reached itself.
    case endInvoke(invokeID: String, token: UInt64, failed: Bool, data: String, source: String?)
}

/// The Mesh router a child session is given by its parent: the child's endpoint attached to that
/// one session.
///
/// What a host installs on a machine it starts, the parent installs on the child it starts,
/// before the child initializes, since an `onEntry` of the child's initial state may already
/// send: the child's `<send type="sce:mesh">` goes to `send` and its `<invoke type="sce:mesh-rpc">`
/// to `invoke`, as `registerMeshRouter` and `registerMeshRpcInvoker` route a machine's. What
/// arrives for the child (SCE_MESH.md §21.4) waits here until the parent drives the child, which
/// takes it with `takeCalls` before the child's macrostep. When the session ends the parent says
/// so with `detach`, after the child has stopped.
public protocol ChildMeshRouter: AnyObject {
    /// A `<send type="sce:mesh">` of the child, answered as a registered router answers.
    func send(_ request: HostSendRequest) -> [HostSendResponse]

    /// An `<invoke type="sce:mesh-rpc">` of the child starting or being cancelled.
    func invoke(_ event: HostInvokeEvent) -> HostInvokeResponse?

    /// What arrived for the child since it was last asked, oldest first.
    func takeCalls() -> [MeshCall]

    /// The session this router was attached to has ended (SCE_MESH.md §21.5): it reached a
    /// top-level `<final>`, or its parent left the invoking state and cancelled it. Called once,
    /// after the child has stopped.
    func detach()

    /// The router of a child this child starts, asked as the parent's factory is asked. Nil by
    /// default: a child given none has its peer sends refused as `error.execution`.
    func childRouter(invokeID: String, document: String) -> (any ChildMeshRouter)?
}

extension ChildMeshRouter {
    public func childRouter(invokeID: String, document: String) -> (any ChildMeshRouter)? {
        nil
    }
}

/// The factory of the routers of a machine's children (SCE_MESH.md §21.3), registered with
/// `Engine.registerChildMeshRouters`: asked, each time an invocation starts a child, with the
/// invoke's id and the stem of the child's document (empty for a child written inline). Nil
/// leaves the child with no router.
public typealias ChildMeshRouters = (_ invokeID: String, _ document: String) -> (any ChildMeshRouter)?

/// What an engine holds of its Mesh routing: the factory of its children's routers, and the
/// router it was given when it is itself a child session. Internal, held by `Engine.meshRouting`.
struct MeshRouting {
    var childRouters: ChildMeshRouters?

    /// The router this machine was given as a child session; nil for one its host started.
    var installed: (any ChildMeshRouter)?
}

extension Engine {
    /// Registers `handler` as the invoker Mesh request/replies reach: the build lowers
    /// `<invoke type="sce:mesh-rpc">` (SCE_MESH.md §9.5) to a host-run invoke of
    /// `meshRpcInvokeType`, with the request's event in the reserved `_mesh_event` param and its
    /// deadline, when the document gave one, in `_mesh_deadline_ms`.
    ///
    /// The invoke half of `registerMeshRouter`. Unregistered, a Mesh request raises
    /// `error.execution` as any host-run invoke without an invoker does; the handler answers with
    /// `completeHostInvoke` or `failHostInvoke`, naming the peer that answered.
    public func registerMeshRpcInvoker(_ handler: @escaping HostInvokeHandler) {
        hostInvokes.meshRpc = handler
    }

    /// Registers the factory of the routers of this machine's children (SCE_MESH.md, Child
    /// Sessions in Mesh; docs/adr/0043).
    ///
    /// A child session started by `<invoke type="scxml">` is given none of this machine's
    /// registrations: its peer sends would leave under this machine's name. `startChildMesh`
    /// asks `factory` instead, each time an invocation starts a child, and installs the answer on
    /// the child before the child initializes. A factory that answers nil, or none registered,
    /// leaves the child with no router: its peer send raises `error.execution` in the child, and
    /// nothing falls back to this machine's router.
    public func registerChildMeshRouters(_ factory: @escaping ChildMeshRouters) {
        meshRouting.childRouters = factory
    }

    /// Asks this machine's factory for the router of `child`, which the invoke `invokeID` is
    /// starting from the document whose stem is `document`, and installs the answer on `child`.
    /// Leaves `child` as it is when no factory is registered or it answers nil. The generated
    /// start of a child calls it before the child initializes.
    public func startChildMesh(of child: any ChildEngine, invokeID: String, document: String) {
        // The invokers a host registered serve every session of the tree (docs/adr/0045, section 3), so the child
        // is handed this machine's registry before its invocations can start, and a registration made later serves
        // it too. Every start and every resume of a child passes here.
        (child as? any SharesHostInvokers)?.shareHostInvokers(hostInvokes.shared)
        guard let router = meshRouting.childRouters?(invokeID, document) else {
            return
        }
        child.installMeshRouter(router)
    }

    /// Installs `router` as this machine's Mesh router, its request invoker and the factory of
    /// its children's routers: the three a host installs on a machine it starts, served by the
    /// one router the parent gave this session.
    func installChildMeshRouter(_ router: any ChildMeshRouter) {
        registerMeshRouter { request in router.send(request) }
        registerMeshRpcInvoker { event in router.invoke(event) }
        meshRouting.childRouters = { invokeID, document in
            router.childRouter(invokeID: invokeID, document: document)
        }
        meshRouting.installed = router
    }

    /// Applies what arrived for this machine on the router its parent gave it, in the order the
    /// router released it. The parent calls it each time it drives this child, before the
    /// child's macrostep. Does nothing for a machine given no router.
    public func applyMeshCalls() {
        guard let router = meshRouting.installed else {
            return
        }
        for call in router.takeCalls() {
            applyMeshCall(call)
        }
    }

    /// Tells this engine one thing its Mesh endpoint has for it: raise an event with the `_event`
    /// the endpoint built, or end a Mesh request. An event name the machine does not declare is
    /// dropped, as the engine drops any such event; an end the engine no longer waits for is
    /// refused by the token check. An answer carries `_event.origin` `mesh://<source>` and origin
    /// type `sce:mesh-rpc` (SCE_MESH.md §9.5), the pair the Mesh transport writes for that peer
    /// (docs/adr/0090, section 2); a deadline the requester reached itself has no peer to name,
    /// and carries none.
    public func applyMeshCall(_ call: MeshCall) {
        switch call {
        case let .raise(name, metadata):
            raiseExternalByNameWithMeta(name, metadata: metadata)
        case let .endInvoke(invokeID, token, failed, data, source):
            let origin = source.map { EventOrigin(processor: meshRpcInvokeType, address: meshOriginScheme + $0) }
            if failed {
                failHostInvoke(
                    processorType: meshRpcInvokeType,
                    invokeID: invokeID,
                    token: token,
                    errorData: data,
                    origin: origin
                )
            } else {
                completeHostInvoke(
                    processorType: meshRpcInvokeType,
                    invokeID: invokeID,
                    token: token,
                    doneData: data,
                    origin: origin
                )
            }
        }
    }

    /// The child session this machine runs has ended: tells the router its parent gave it, once.
    /// Called after the child reached a top-level `<final>`, or after it was stopped because the
    /// invoking state exited, so a send the child made while stopping was handed over first.
    public func detachMesh() {
        guard let router = meshRouting.installed else {
            return
        }
        meshRouting.installed = nil
        router.detach()
    }
}
