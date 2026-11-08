// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host half, case for case with backends/rust/mesh/src/endpoint.rs.

package com.sce.mesh

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.runtime.EventOrigin
import com.sce.runtime.IoProcessors
import com.sce.runtime.StateMachineEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

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

        /** The wall clock a fixed distance ahead of the monotonic one, so a wire deadline and a kept one can be told apart. */
        override fun nowUnixMs(): Long = 1_000_000 + now

        override fun envelopeId(): ByteArray {
            ids += 1
            return ByteArray(16) { ids.toByte() }
        }

        override fun jitterDraw(): Long = 0
    }

    /** The events [this] endpoint queued, which in these tests are all it queued. */
    private fun Endpoint.takeEvents(): List<EngineEvent> =
        takeCalls().map { (it as? EngineCall.Raise ?: error("expected only events, got $it")).event }

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
                    buffer = OutboundBuffer(maxPending = 4u, maxAgeMs = 0),
                    retry = retry,
                    stampSequence = false,
                    delivery = Delivery(dedup = true, ordered = false),
                    responders = listOf(peer),
                    deadlineMs = null,
                    replyEvents = emptyList(),
                    causal = null,
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
        // The pair the transport writes for the peer (docs/adr/0090), and no
        // sender: the peer is no session of this tree.
        assertEquals(EventOrigin(IoProcessors.SCXML_PROCESSOR, "mesh://ecu"), event.metadata.origin)
        assertEquals(null, event.metadata.sender)
        assertEquals("s-7", event.metadata.sendId)
    }

    @Test
    fun aLinkMayBeForbiddenEvents() {
        val ecu = Host("ecu", "hmi", null)
        ecu.endpoint.send(send("go", "s-1"))
        val go = ecu.transport.sent.removeLast().second
        ecu.endpoint.send(send("stop", "s-2"))
        val stop = ecu.transport.sent.removeLast().second

        val hmi = Host("hmi", "ecu", null)
        hmi.endpoint.forbidEventsFrom("ecu", listOf("go"))
        hmi.endpoint.receive("ecu", go)
        hmi.endpoint.receive("ecu", stop)
        assertEquals(listOf("stop"), hmi.endpoint.takeEvents().map { it.name })
        assertEquals(listOf<RouterError>(RouterError.EventForbidden("ecu", "go")), hmi.endpoint.takeHostErrors())

        hmi.endpoint.forbidEventsFrom("ecu", emptyList())
        val again = Host("hmi", "ecu", null)
        again.endpoint.receive("ecu", go)
        assertEquals(1, again.endpoint.takeEvents().size)
        assertTrue(again.endpoint.takeHostErrors().isEmpty())
    }

    /**
     * What decides an event's admission is shown the link, the name and the data the sender wrote;
     * one it does not admit never reaches the machine and the host is told why, one it admits goes
     * through with what the sealer wrote taken out again.
     */
    @Test
    fun anAdmissionDecidesWhatReachesTheMachine() {
        val ecu = Host("ecu", "hmi", null)
        ecu.endpoint.setSealer { target, event, data ->
            assertEquals("#hmi", target)
            if (event == "never") Sealed.Withhold("this one does not leave") else Sealed.Leave("$data+seal")
        }
        ecu.endpoint.send(send("go", "s-1"))
        val go = ecu.transport.sent.removeLast().second
        ecu.endpoint.send(send("stop", "s-2"))
        val stop = ecu.transport.sent.removeLast().second

        val hmi = Host("hmi", "ecu", null)
        hmi.endpoint.setAdmission { peer, event, data ->
            assertEquals("ecu", peer)
            if (event == "go") {
                Admit.Allow(data.removeSuffix("+seal"))
            } else {
                Admit.Refuse("$event is not for this link")
            }
        }
        hmi.endpoint.receive("ecu", go)
        hmi.endpoint.receive("ecu", stop)
        val events = hmi.endpoint.takeEvents()
        assertEquals(listOf("go"), events.map { it.name })
        assertEquals("""{"a":1}""", events.single().metadata.data)
        assertEquals(
            listOf<RouterError>(RouterError.EventNotAdmitted("ecu", "stop", "stop is not for this link")),
            hmi.endpoint.takeHostErrors(),
        )

        // A send the sealer says must not leave is dropped, told to the host, and nothing goes in its place.
        ecu.endpoint.send(send("never", "s-3"))
        assertTrue(ecu.transport.sent.isEmpty())
        assertEquals(
            listOf<RouterError>(RouterError.EventNotSealed("#hmi", "never", "this one does not leave")),
            ecu.endpoint.takeHostErrors(),
        )
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
        assertEquals("mesh://ecu", event.metadata.origin?.address)
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

    /**
     * docs/adr/0085, section 3, "offer and verdict": a causal peer's verdicts reach the host, a
     * change leaves only once the host reports its journal record committed, and a link that comes
     * back sends this copy's vector under an id the endpoint drew. Red if the endpoint drops a
     * verdict, if it releases a change at its verdict, or if it performs no announcement.
     */
    @Test
    fun aCausalChangeLeavesWhenTheHostCommitsItsVerdict() {
        val transport = Recorder()
        val environment = Fixed()
        val router = Router("list", 8u, 50)
        router.addPeer(
            "people",
            PeerConfig(
                transport = "wss",
                buffer = null,
                retry = null,
                stampSequence = false,
                delivery = Delivery(dedup = false, ordered = false),
                responders = listOf("people"),
                deadlineMs = null,
                replyEvents = emptyList(),
                causal = CausalGroup("people", 0uL, listOf("phone/phone_list", "laptop/laptop_list")),
            ),
        )
        val endpoint = Endpoint(router, transport, environment)
        val phone = Replica.fromBytes(ByteArray(16) { 1 })
        assertNull(endpoint.startCausal("people", CausalStart.NewIncarnation(phone), 1, 100))
        val edit = StateMachineEngine.HostSendRequest(
            processorType = MESH_PROCESSOR_TYPE,
            eventName = "edit",
            target = "#people",
            sendId = "",
            eventData = """{"a":1}""",
        )

        endpoint.send(edit)
        assertTrue(transport.sent.isEmpty(), "nothing leaves before the commit")
        val verdict = endpoint.takeVerdicts().single() as? CausalVerdict.Accepted ?: fail("expected one acceptance")
        assertEquals(CausalVerdict.Accepted("people", Dot(phone, 1uL)), verdict)
        endpoint.committed("people", verdict.dot)
        assertEquals(1, transport.sent.size)

        // The log has room for one change not yet stable, and the laptop has
        // not said what it delivered.
        endpoint.send(edit)
        assertEquals(
            listOf<CausalVerdict>(CausalVerdict.Refused("people", Refusal.NO_ROOM_FOR_HISTORY)),
            endpoint.takeVerdicts(),
        )
        assertEquals(1, transport.sent.size)

        endpoint.peerReady("people")
        assertEquals(2, transport.sent.size)
        val (peer, bytes) = transport.sent.last()
        assertEquals("people", peer)
        val vector = Envelope.decode(SceCursor(bytes)) ?: fail("the vector is an envelope")
        assertEquals(PatternKind.CAUSAL_VECTOR, vector.pattern)
        assertEquals(listOf(phone.entry(1uL)), vector.causal_vector)
        // Each send drew an id, the refused one too; the vector drew the third.
        assertTrue(vector.id.contentEquals(ByteArray(16) { 3 }))
    }

    @Test
    fun whatOnlyTheHostCanActOnNeverReachesTheDocument() {
        val ecu = Host("ecu", "hmi", null)
        ecu.endpoint.receive("stranger", byteArrayOf(0xFF.toByte()))
        assertTrue(ecu.endpoint.takeEvents().isEmpty())
        assertEquals(listOf<RouterError>(RouterError.UnknownPeer("stranger")), ecu.endpoint.takeHostErrors())
    }
}
