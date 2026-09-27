// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host half, case for case with backends/rust/mesh/src/endpoint.rs.

package com.sce.mesh

import com.sce.runtime.IoProcessors
import com.sce.runtime.StateMachineEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EndpointTest {
    /** A transport that records what it sent and fails as told. */
    private class Recorder : Transport {
        val sent = mutableListOf<Pair<String, ByteArray>>()
        val failures = ArrayDeque<TransportFailure>()

        override fun transmit(peer: String, bytes: ByteArray): TransportFailure? {
            failures.removeFirstOrNull()?.let { return it }
            sent += peer to bytes
            return null
        }
    }

    /** A clock the test moves, ids that count, and no jitter. */
    private class Fixed : Environment {
        var now = 0L
        private var ids = 0

        override fun nowMs(): Long = now

        override fun envelopeId(): ByteArray {
            ids += 1
            return ByteArray(16) { ids.toByte() }
        }

        override fun jitterDraw(): Long = 0
    }

    private class Host(machine: String, peer: String, retry: RetryPolicy?) {
        val transport = Recorder()
        val environment = Fixed()
        val endpoint: Endpoint

        init {
            val router = Router(machine, 8u, 50)
            router.addPeer(
                peer,
                PeerConfig(
                    transport = "wss",
                    maxPending = 4u,
                    maxAgeMs = 0,
                    retry = retry,
                    stampSequence = false,
                    delivery = Delivery(dedup = true, ordered = false),
                ),
            )
            endpoint = Endpoint(router, transport, environment)
            endpoint.peerReady(peer)
        }
    }

    private fun send(event: String, sendId: String) = StateMachineEngine.HostSendRequest(
        processorType = MESH_PROCESSOR_TYPE,
        eventName = event,
        target = "#hmi",
        sendId = sendId,
        eventData = """{"a":1}""",
    )

    private fun failure(retryable: Boolean) = TransportFailure(retryable, "reset")

    @Test
    fun aDeliveredEventCarriesTheFieldsTheCppCoreGivesIt() {
        val ecu = Host("ecu", "hmi", null)
        val hmi = Host("hmi", "ecu", null)
        ecu.endpoint.send(send("go", "s-7"))
        val (peer, bytes) = ecu.transport.sent.removeLast()
        assertEquals("hmi", peer)
        hmi.endpoint.receive("ecu", bytes)

        val event = hmi.endpoint.takeEvents().single()
        assertEquals("go", event.name)
        assertEquals("""{"a":1}""", event.metadata.data)
        assertEquals("external", event.metadata.type)
        assertEquals("mesh://ecu", event.metadata.origin)
        assertEquals(IoProcessors.SCXML_PROCESSOR, event.metadata.originType)
        assertEquals("s-7", event.metadata.sendId)
    }

    @Test
    fun aSendWithoutAnIdArrivesWithoutOne() {
        val ecu = Host("ecu", "hmi", null)
        val hmi = Host("hmi", "ecu", null)
        ecu.endpoint.send(send("go", ""))
        hmi.endpoint.receive("ecu", ecu.transport.sent.removeLast().second)
        assertEquals("", hmi.endpoint.takeEvents().single().metadata.sendId)
    }

    @Test
    fun aFailedSendWithNoRetryPolicyIsLostAsSendFailed() {
        val ecu = Host("ecu", "hmi", null)
        ecu.transport.failures.addLast(failure(true))
        ecu.endpoint.send(send("go", "s-1"))

        val event = ecu.endpoint.takeEvents().single()
        assertEquals("error.communication", event.name)
        assertEquals(
            """{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss","transport_error":"reset"}""",
            event.metadata.data,
        )
        // Raised as the C++ core raises it: an envelope from its own machine.
        assertEquals("mesh://ecu", event.metadata.origin)
        assertEquals("", event.metadata.sendId)
    }

    @Test
    fun aRetryableFailureIsSentAgainAfterItsBackoff() {
        val ecu = Host("ecu", "hmi", RetryPolicy(2u, 100, 2.0, 1000, 0))
        ecu.transport.failures.addLast(failure(true))
        ecu.endpoint.send(send("go", "s-1"))
        assertTrue(ecu.transport.sent.isEmpty())
        assertTrue(ecu.endpoint.takeEvents().isEmpty())

        ecu.environment.now = 99
        ecu.endpoint.tick()
        assertTrue(ecu.transport.sent.isEmpty(), "sent before its backoff")

        ecu.environment.now = 100
        ecu.endpoint.tick()
        assertEquals(1, ecu.transport.sent.size)
        assertTrue(ecu.endpoint.takeEvents().isEmpty())
    }

    @Test
    fun retriesThatRunOutAreDeliveryExhausted() {
        val ecu = Host("ecu", "hmi", RetryPolicy(1u, 10, 1.0, 10, 0))
        ecu.transport.failures.addLast(failure(true))
        ecu.transport.failures.addLast(failure(true))
        ecu.endpoint.send(send("go", "s-1"))
        ecu.environment.now = 10
        ecu.endpoint.tick()

        assertEquals(
            """{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"reset","attempts":2}""",
            ecu.endpoint.takeEvents().single().metadata.data,
        )
    }

    @Test
    fun aTerminalFailureIsNotRetried() {
        val ecu = Host("ecu", "hmi", RetryPolicy(5u, 10, 1.0, 10, 0))
        ecu.transport.failures.addLast(failure(false))
        ecu.endpoint.send(send("go", "s-1"))
        assertTrue(ecu.endpoint.takeEvents().single().metadata.data.contains(""""attempts":1"""))
        ecu.environment.now = 1000
        ecu.endpoint.tick()
        assertTrue(ecu.transport.sent.isEmpty())
    }

    @Test
    fun whatOnlyTheHostCanActOnNeverReachesTheDocument() {
        val ecu = Host("ecu", "hmi", null)
        ecu.endpoint.receive("stranger", byteArrayOf(0xFF.toByte()))
        assertTrue(ecu.endpoint.takeEvents().isEmpty())
        assertEquals(listOf<RouterError>(RouterError.UnknownPeer("stranger")), ecu.endpoint.takeHostErrors())
    }
}
