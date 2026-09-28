// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Kotlin core against the Rust core over one real WebSocket (SCE_MESH.md
// §mesh-18). The Rust side is backends/rust/mesh/examples/wss_peer.rs, a
// server machine that answers each `ping` with a `pong` carrying the same
// `_event.data`; the build hands its path in as `sce.mesh.wssPeer`.
//
// What it proves is what the two cores' shared literal tests cannot: the Rust
// core decodes an envelope the Kotlin core encoded, and the Kotlin core
// decodes one the Rust core encoded, on the wire the binding fixes. Both
// sides build their routers from the peer tables the build generated from
// one deployment (tests/mesh/wss_loopback/deploy.yaml, SCE_MESH.md
// §mesh-19), so the configuration they agree on is the one deploy.yaml says.

package com.sce.mesh

import com.sce.generated.client.ClientMeshPeers
import com.sce.runtime.StateMachineEngine
import java.io.File
import java.time.Duration
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlin.test.fail
import okhttp3.OkHttpClient

class CrossLanguageLoopbackTest {
    private class Counting : Environment {
        private var now = 0L
        private var ids = 0

        override fun nowMs(): Long = ++now

        override fun envelopeId(): ByteArray {
            ids += 1
            return ByteArray(16) { (0x30 + ids).toByte() }
        }

        override fun jitterDraw(): Long = 0
    }

    private fun <T> LinkedBlockingQueue<T>.next(): T = poll(10, TimeUnit.SECONDS) ?: fail("nothing within ten seconds")

    @Test
    fun theKotlinAndRustCoresReadEachOthersEnvelopes() {
        val path = System.getProperty("sce.mesh.wssPeer") ?: fail("the build did not name the Rust peer (sce.mesh.wssPeer)")
        assertTrue(
            File(path).canExecute(),
            "the Rust peer is not built at $path — `cargo build -p sce-rust-mesh --features wss --example wss_peer`",
        )
        val peer = ProcessBuilder(path).redirectError(ProcessBuilder.Redirect.INHERIT).start()
        try {
            val stdout = peer.inputStream.bufferedReader()
            val listening = stdout.readLine() ?: fail("the Rust peer exited before it listened")
            val port = listening.removePrefix("LISTENING ").toIntOrNull() ?: fail("unexpected first line: $listening")

            // The machine as tests/mesh/wss_loopback/deploy.yaml binds it, from the
            // table the build generated. The server's port is the one thing the
            // deployment cannot know: the Rust peer takes an ephemeral one.
            val machine = ClientMeshPeers.MACHINE
            val server = machine.peer("server") ?: fail("the deployment binds no #server")
            val dial = assertIs<PeerLink.WssDial>(server.link)
            val base = dial.url.substringBeforeLast(':') + ":$port"
            val keepalive = dial.keepaliveMs?.let { Duration.ofMillis(it.toLong()) } ?: DEFAULT_KEEPALIVE

            val events = LinkedBlockingQueue<LinkEvent>()
            val link = WssClient(OkHttpClient(), base, machine.name, server.name, keepalive) { events += it }
            val endpoint = Endpoint(machine.router(), link, Counting())
            link.connect()
            deliver(endpoint, events.next().also { assertEquals(LinkEvent.Ready("server"), it) })

            // Text the escaping rules would get wrong if either side had its own:
            // a quote, a backslash, a C0 control as JSON writes it, and a two-
            // and a three-byte UTF-8 character (U+00E9, U+20AC).
            val data = """{"n":1,"s":"q\"\\ \u0001 é €"}"""
            endpoint.send(
                StateMachineEngine.HostSendRequest(
                    processorType = MESH_PROCESSOR_TYPE,
                    eventName = "ping",
                    target = "#server",
                    sendId = "ask-1",
                    eventData = data,
                ),
            )
            assertTrue(endpoint.takeHostErrors().isEmpty())

            val reply = events.next()
            assertIs<LinkEvent.Received>(reply)
            deliver(endpoint, reply)
            val pong = endpoint.takeEvents().single()
            assertEquals("pong", pong.name)
            assertEquals(data, pong.metadata.data)
            assertEquals("reply-1", pong.metadata.sendId)
            assertEquals("mesh://server", pong.metadata.origin)

            link.close()
            assertTrue(peer.waitFor(10, TimeUnit.SECONDS), "the Rust peer did not exit after the link closed")
            assertEquals("DONE 1", stdout.readLine())
            assertEquals(0, peer.exitValue())
        } finally {
            peer.destroyForcibly()
        }
    }
}
