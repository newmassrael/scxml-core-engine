// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Mesh router a child session is given by its parent (SCE_MESH.md, Child
// Sessions in Mesh; docs/adr/0043). The Kotlin twin of the Rust runtime's
// `ChildMeshRouter`, `MeshCall` and `mesh_event_metadata`.

package com.sce.runtime

/**
 * `_event.origin` of a Mesh-delivered event is this scheme and the sending
 * machine's name (SCE_MESH.md §mesh-10.7) — the C++ core's `kMeshOriginScheme`.
 */
const val MESH_ORIGIN_SCHEME = "mesh://"

/**
 * The pair the Mesh transport writes for an event a peer's envelope delivers
 * (docs/adr/0090, section 2): the SCXML Event I/O Processor, since every
 * envelope is SCE's own inter-SCXML traffic (SCE_MESH.md §mesh-10.7), at the
 * address `mesh://<source>` the transport reaches the peer [source] by. The
 * Kotlin twin of the C++ `SCE::Mesh::meshPeerOrigin`.
 */
fun meshPeerOrigin(source: String): EventOrigin =
    EventOrigin(IoProcessors.SCXML_PROCESSOR, MESH_ORIGIN_SCHEME + source)

/**
 * The pair the Mesh transport writes for the answer to an `<invoke
 * type="sce:mesh-rpc">` (SCE_MESH.md, the runtime mapping of a request's reply
 * to `_event`): the request's own type at the address `mesh://<replier>` of the
 * peer that answered. The Kotlin twin of the C++ `SCE::Mesh::meshRpcReplyOrigin`.
 */
fun meshRpcReplyOrigin(replier: String): EventOrigin =
    EventOrigin(MESH_RPC_INVOKE_TYPE, MESH_ORIGIN_SCHEME + replier)

/**
 * The `_event` of an event that arrived over Mesh from [source], as SCE_MESH.md
 * §mesh-10.7 tabulates it: external, the transport's pair for the peer
 * ([meshPeerOrigin]: origin `mesh://<source>`, the SCXML processor as origin
 * type), and the send id the envelope carried. No sender: the peer is no
 * session of this tree.
 *
 * The one place it is built: a host core's endpoint builds what it hands a
 * top-level machine with it, and what it hands a child session, so a child
 * cannot tell a peer's event from the one its parent would have been given.
 */
fun meshEventMetadata(data: String, source: String, sendId: String): EventMetadata =
    EventMetadata(
        data = data,
        type = "external",
        sendId = sendId,
        origin = meshPeerOrigin(source),
    )

/**
 * One thing a Mesh endpoint has for the engine it serves, in the order the
 * endpoint came to know it: what [StateMachineEngine.applyMeshCall] applies.
 */
sealed class MeshCall {
    /** Raise [name] on the external queue with the `_event` the endpoint built for it. */
    data class Raise(val name: String, val metadata: EventMetadata) : MeshCall()

    /**
     * End the start [token] of the Mesh request [invokeId] — with `done.invoke`,
     * or with `error.invoke` when [failed] — carrying [data] as `_event.data`.
     * [source] is the machine that answered, `null` for a deadline the
     * requester reached itself.
     */
    data class EndInvoke(
        val invokeId: String,
        val token: Long,
        val failed: Boolean,
        val data: String,
        val source: String?,
    ) : MeshCall()
}

/**
 * The Mesh router a child session is given by its parent: the child's endpoint
 * attached to that one session.
 *
 * What a host installs on a machine it starts, the parent installs on the child
 * it starts, before the child initializes, since an `onEntry` of the child's
 * initial state may already send: the child's `<send type="sce:mesh">` goes to
 * [send] and its `<invoke type="sce:mesh-rpc">` to [invoke], as
 * [StateMachineEngine.registerMeshRouter] and
 * [StateMachineEngine.registerMeshRpcInvoker] route a machine's. What arrives for
 * the child (§mesh-21.4) waits here until the parent drives the child, which takes it with
 * [takeCalls] before the child's macrostep. When the session ends the parent
 * says so with [detach], after the child has stopped.
 */
interface ChildMeshRouter {
    /** A `<send type="sce:mesh">` of the child, answered as a registered router answers. */
    fun send(request: StateMachineEngine.HostSendRequest): List<StateMachineEngine.HostSendResponse>

    /** An `<invoke type="sce:mesh-rpc">` of the child starting or being cancelled. */
    fun invoke(event: StateMachineEngine.HostInvokeEvent): StateMachineEngine.HostInvokeResponse?

    /** What arrived for the child since it was last asked, oldest first. */
    fun takeCalls(): List<MeshCall>

    /**
     * The session this router was attached to has ended (§mesh-21.5): it reached a top-level
     * `<final>`, or its parent left the invoking state and cancelled it. Called
     * once, after the child has stopped.
     */
    fun detach()

    /**
     * The router of a child this child starts, asked as the parent's factory is
     * asked. `null` by default: a child given none has its peer sends refused as
     * `error.execution`.
     */
    fun childRouter(invokeId: String, document: String): ChildMeshRouter? = null
}

/**
 * The factory of the routers of a machine's children (§mesh-21.3), registered
 * with [StateMachineEngine.registerChildMeshRouters]: asked, each time an invocation
 * starts a child, with the invoke's id and the stem of the child's document
 * (empty for a child written inline). `null` leaves the child with no router.
 */
typealias ChildMeshRouters = (invokeId: String, document: String) -> ChildMeshRouter?
