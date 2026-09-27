// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a link — one connection to one peer — tells its host, and how the host
// hands that to its [Endpoint]. The same four events the Rust binding's links
// report (backends/rust/mesh/src/wss.rs); a transport adapter produces them on
// its own thread and the host delivers them on the engine's.

package com.sce.mesh

/** What a link tells its host. */
sealed class LinkEvent {
    /** The connection opened: [peer]'s binding is ready. */
    data class Ready(val peer: String) : LinkEvent()

    /** A binary message from [peer]: one envelope's bytes. */
    class Received(val peer: String, val bytes: ByteArray) : LinkEvent()

    /** An envelope queued for [peer] that the link closed before writing. */
    class Unsent(val peer: String, val bytes: ByteArray) : LinkEvent()

    /** The link to [peer] closed, failed, or stopped answering pings; [reason] says which. */
    data class Lost(val peer: String, val reason: String) : LinkEvent()
}

/** Hand a link's event to the endpoint it concerns. */
fun deliver(endpoint: Endpoint, event: LinkEvent) {
    when (event) {
        is LinkEvent.Ready -> endpoint.peerReady(event.peer)
        is LinkEvent.Received -> endpoint.receive(event.peer, event.bytes)
        is LinkEvent.Unsent -> endpoint.transmitFailed(
            event.peer,
            event.bytes,
            TransportFailure(retryable = true, message = "the link closed before the envelope was written"),
        )
        is LinkEvent.Lost -> endpoint.peerNotReady(event.peer)
    }
}

/**
 * The path a client opens, before its own machine name (SCE_MESH.md
 * §mesh-18.1), and whether a name is a path segment as it stands: RFC 3986's
 * unreserved characters, so no encoding can make two names one.
 */
const val WSS_PATH_PREFIX = "/sce-mesh/1/"

fun isPathSegment(name: String): Boolean =
    name.isNotEmpty() && name.all { it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it in "-._~" }
