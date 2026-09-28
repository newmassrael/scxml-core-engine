// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine's Mesh peers as its deployment binds them (SCE_MESH.md §mesh-19):
// the shape `sce-codegen generate --deploy --lang kotlin` fills in
// `<Machine>MeshPeers.kt`, and the router it builds. The Kotlin twin of
// backends/rust/mesh/src/peers.rs.
//
// The generated file is data only. Every decision about a binding — whether
// its envelopes are deduplicated, ordered, buffered, retried — was taken by
// the build from deploy.yaml, with the same function that decides it for the
// C++ router, and lands here as a [PeerConfig]. What a host still does by
// hand is open the connections each [PeerLink] names.

package com.sce.mesh

/**
 * How the link to a peer is opened. The core never reads it; the host's transport adapter
 * does. (Not [Binding], which is what a §mesh-16.7 row names.)
 */
sealed class PeerLink {
    /** The §mesh-18.3 ping interval, or `null` for the binding's 30 s. */
    abstract val keepaliveMs: UInt?

    /** §mesh-18.1: this machine dials the peer at [url] (`wss://<host>[:<port>][<base>]`). */
    data class WssDial(val url: String, override val keepaliveMs: UInt?) : PeerLink()

    /** §mesh-18.1: the peer dials this machine. */
    data class WssAccept(override val keepaliveMs: UInt?) : PeerLink()
}

/** One peer: the name `#<name>` addresses, how the core treats envelopes to and from it, and how it is reached. */
data class Peer(val name: String, val config: PeerConfig, val link: PeerLink)

/**
 * One machine's side of a deployment: its [name] (the `source` its envelopes
 * carry), deploy.yaml's `dedup.window_size` ([dedupWindow], §mesh-10.5) and
 * `ordering.gap_timeout_ms` ([gapTimeoutMs], §mesh-10.6.1), and every peer
 * it is bound to.
 */
data class MeshMachine(
    val name: String,
    val dedupWindow: UInt,
    val gapTimeoutMs: Long,
    val peers: List<Peer>,
) {
    /** A router holding every peer this machine is bound to. */
    fun router(): Router {
        val router = Router(name, dedupWindow, gapTimeoutMs)
        for (peer in peers) router.addPeer(peer.name, peer.config)
        return router
    }

    /** The peer named [name], if the machine is bound to one. */
    fun peer(name: String): Peer? = peers.firstOrNull { it.name == name }
}
