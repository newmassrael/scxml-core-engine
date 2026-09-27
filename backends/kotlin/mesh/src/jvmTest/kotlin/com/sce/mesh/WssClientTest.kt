// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The OkHttp client of the WebSocket binding (SCE_MESH.md §mesh-18) against a
// real WebSocket server, the Kotlin twin of the Rust binding's socket tests.

package com.sce.mesh

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.runtime.StateMachineEngine
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlin.test.fail
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString

class WssClientTest {
    private val server = MockWebServer()
    private val serverSockets = LinkedBlockingQueue<WebSocket>()
    private val serverReceived = LinkedBlockingQueue<ByteArray>()
    private val clientEvents = LinkedBlockingQueue<LinkEvent>()

    /** The server machine's side of the link, as a peer writing and reading envelopes. */
    private val serverListener = object : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: okhttp3.Response) {
            serverSockets += webSocket
        }

        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
            serverReceived += bytes.toByteArray()
        }
    }

    @BeforeTest
    fun start() {
        server.start()
    }

    @AfterTest
    fun stop() {
        server.close()
    }

    private fun <T> LinkedBlockingQueue<T>.next(): T = poll(5, TimeUnit.SECONDS) ?: fail("nothing within five seconds")

    private class Counting : Environment {
        private var now = 0L
        private var ids = 0

        override fun nowMs(): Long = ++now

        override fun envelopeId(): ByteArray {
            ids += 1
            return ByteArray(16) { ids.toByte() }
        }

        override fun jitterDraw(): Long = 0
    }

    private fun router(machine: String, peer: String): Router =
        Router(machine, 8u, 50).also {
            it.addPeer(
                peer,
                PeerConfig(
                    transport = "wss",
                    maxPending = 8u,
                    maxAgeMs = 0,
                    retry = null,
                    stampSequence = false,
                    delivery = Delivery(dedup = true, ordered = false),
                ),
            )
        }

    private fun client() = WssClient(
        OkHttpClient(),
        base = server.url("/").toString(),
        own = "client",
        server = "server",
        events = { clientEvents += it },
    )

    private fun request(event: String) = StateMachineEngine.HostSendRequest(
        processorType = MESH_PROCESSOR_TYPE,
        eventName = event,
        target = "#server",
        sendId = "s-1",
        eventData = """{"v":1}""",
    )

    @Test
    fun envelopesCrossARealSocketBothWays() {
        server.enqueue(MockResponse.Builder().webSocketUpgrade(serverListener).build())
        val link = client()
        val endpoint = Endpoint(router("client", "server"), link, Counting())
        link.connect()

        assertEquals(LinkEvent.Ready("server"), clientEvents.next())
        assertEquals("/sce-mesh/1/client", server.takeRequest().url.encodedPath)
        deliver(endpoint, LinkEvent.Ready("server"))

        // Client to server: the core's envelope, as the socket carried it.
        endpoint.send(request("go"))
        val sent = Envelope.decode(SceCursor(serverReceived.next())) ?: fail("not an envelope")
        assertEquals("go", sent.event_type)
        assertEquals("client", sent.source)
        assertEquals("s-1", sent.subject)

        // Server to client: an envelope the server machine's core wrote.
        val serverRouter = router("server", "client")
        assertTrue((serverRouter.peerReady("client", 0) as Routed.Done).effects.isEmpty())
        val reply = (serverRouter.send(
            request("done").copy(target = "#client", sendId = ""),
            ByteArray(16) { 9 },
            0,
        ) as Routed.Done).effects.single() as Effect.Transmit
        serverSockets.next().send(reply.bytes.toByteString())
        val received = clientEvents.next()
        assertIs<LinkEvent.Received>(received)
        deliver(endpoint, received)

        val event = endpoint.takeEvents().single()
        assertEquals("done", event.name)
        assertEquals("mesh://server", event.metadata.origin)
        assertTrue(endpoint.takeHostErrors().isEmpty())
        link.close()
    }

    @Test
    fun aRefusedUpgradeIsReportedAsALostLinkNotAReadyOne() {
        server.enqueue(MockResponse.Builder().code(404).build())
        client().connect()
        val event = clientEvents.next()
        assertIs<LinkEvent.Lost>(event)
        assertEquals("server", event.peer)
    }

    @Test
    fun aClosedLinkEndsReadinessAndTheCoreRaisesRowOne() {
        server.enqueue(MockResponse.Builder().webSocketUpgrade(serverListener).build())
        val link = client()
        val endpoint = Endpoint(router("client", "server"), link, Counting())
        link.connect()
        deliver(endpoint, clientEvents.next())

        serverSockets.next().close(1000, "the server is done")
        val lost = clientEvents.next()
        assertIs<LinkEvent.Lost>(lost)
        deliver(endpoint, lost)
        assertEquals(
            """{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"server","transport":"wss"}""",
            endpoint.takeEvents().single().metadata.data,
        )
    }

    @Test
    fun aNameThatIsNotAPathSegmentCannotDial() {
        val refused = runCatching {
            WssClient(OkHttpClient(), server.url("/").toString(), "a/b", "server") {}
        }
        assertTrue(refused.isFailure)
    }
}
