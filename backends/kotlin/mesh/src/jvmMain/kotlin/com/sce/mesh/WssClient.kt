// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The WebSocket binding's client (SCE_MESH.md §mesh-18) over OkHttp: the
// socket plumbing between one server link and the router core, and nothing
// else. It dials `<base>/sce-mesh/1/<own>`, reports what happens on the link
// as [LinkEvent]s, and is the [Transport] the core's sends reach the socket
// through.
//
// OkHttp calls back on its own threads, so [events] runs there; the host
// posts each event to the thread that steps its engine and calls [deliver]
// there, which keeps the endpoint confined to that thread.

package com.sce.mesh

import java.time.Duration
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString

/** The keepalive interval when the deployment sets none (SCE_MESH.md §mesh-18.3). */
val DEFAULT_KEEPALIVE: Duration = Duration.ofSeconds(30)

/**
 * One link to the server machine [server] at [base] (a `ws://` or `wss://`
 * URL before the binding's path), as the machine [own]. [client] is the host's
 * OkHttp client — its TLS settings and interceptors (an authentication header,
 * §mesh-18.1) are the link's; this adds only the keepalive ping.
 */
class WssClient(
    client: OkHttpClient,
    private val base: String,
    private val own: String,
    private val server: String,
    keepalive: Duration = DEFAULT_KEEPALIVE,
    private val events: (LinkEvent) -> Unit,
) : Transport {
    init {
        require(isPathSegment(own)) { "'$own' is not a path segment as it stands (SCE_MESH.md §mesh-18.1)" }
    }

    // OkHttp pings on this interval and fails the socket when a pong does not
    // come back within it (§mesh-18.3).
    private val http = client.newBuilder().pingInterval(keepalive).build()
    private val lock = Any()

    /** The socket last dialled, open or not; callbacks from an older one are ignored. */
    private var current: WebSocket? = null

    /** Whether [current] completed its upgrade. */
    private var open = false

    /**
     * What this link handed OkHttp and OkHttp may not have written yet, oldest
     * first. OkHttp keeps its own outgoing queue and never hands a message
     * back, but it reports the bytes still in it ([WebSocket.queueSize]), and
     * those are the newest messages — so the tail of this list whose sizes
     * add up to that count is what a closing link leaves unwritten.
     */
    private val handed = ArrayDeque<ByteArray>()
    private var handedBytes = 0L

    /** Dial the server. A client reconnects by calling this again, on its own backoff (§mesh-18.3). */
    fun connect() {
        val url = base.trimEnd('/') + WSS_PATH_PREFIX + own
        synchronized(lock) {
            open = false
            current = http.newWebSocket(Request.Builder().url(url).build(), Listener())
        }
    }

    /** Close the link as a client going away; its loss is reported as any other. */
    fun close() {
        synchronized(lock) { current }?.close(1001, "the client is going away")
    }

    override fun transmit(peer: String, bytes: ByteArray): TransportFailure? {
        synchronized(lock) {
            val socket = current?.takeIf { open }
                ?: return TransportFailure(retryable = true, message = "no link to $peer")
            if (!socket.send(bytes.toByteString())) {
                return TransportFailure(retryable = true, message = "the link to $peer closed")
            }
            prune(socket)
            handed.addLast(bytes)
            handedBytes += bytes.size
            return null
        }
    }

    /** Forget the messages OkHttp has written: everything but the unwritten tail. */
    private fun prune(open: WebSocket) {
        val unwritten = open.queueSize()
        while (handed.isNotEmpty() && handedBytes - handed.first().size >= unwritten) {
            handedBytes -= handed.removeFirst().size
        }
    }

    /**
     * The link ended — or never opened, when the upgrade was refused: report
     * what it left unwritten, then the loss, once.
     */
    private fun lost(webSocket: WebSocket, reason: String) {
        val unsent = synchronized(lock) {
            if (current !== webSocket) return
            current = null
            open = false
            prune(webSocket)
            // What `prune` kept is at least the unwritten tail; a message
            // partly written counts as unsent, since the peer cannot read it.
            handed.toList().also {
                handed.clear()
                handedBytes = 0
            }
        }
        for (bytes in unsent) events(LinkEvent.Unsent(server, bytes))
        events(LinkEvent.Lost(server, reason))
    }

    private inner class Listener : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            synchronized(lock) {
                if (current !== webSocket) return
                open = true
                handed.clear()
                handedBytes = 0
            }
            events(LinkEvent.Ready(server))
        }

        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
            if (synchronized(lock) { current === webSocket && open }) {
                events(LinkEvent.Received(server, bytes.toByteArray()))
            }
        }

        // §mesh-18.2: text is not an envelope.
        override fun onMessage(webSocket: WebSocket, text: String) {
            webSocket.close(1003, "an envelope is a binary message")
            lost(webSocket, "the peer sent a text message")
        }

        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
            webSocket.close(1000, null)
            lost(webSocket, "the peer closed the link ($code)")
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            lost(webSocket, "the link closed ($code)")
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            val why = response?.let { "the upgrade was answered ${it.code}" } ?: (t.message ?: t.toString())
            lost(webSocket, why)
        }
    }
}
