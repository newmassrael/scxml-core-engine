// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core, case for case with backends/rust/mesh/src/router.rs.

package com.sce.mesh

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.runtime.StateMachineEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

class RouterTest {
    /** A binding with ordering required on a transport that does not order. */
    private val ordered = PeerConfig(
        transport = "wss",
        maxPending = 4u,
        maxAgeMs = 0,
        retry = null,
        stampSequence = true,
        delivery = Delivery(dedup = true, ordered = true),
    )
    private val unordered = ordered.copy(stampSequence = false, delivery = Delivery(dedup = true, ordered = false))

    private fun request(target: String, event: String, data: String, sendId: String = "send.1") =
        StateMachineEngine.HostSendRequest(
            processorType = MESH_PROCESSOR_TYPE,
            eventName = event,
            target = target,
            sendId = sendId,
            eventData = data,
        )

    private fun id(n: Int) = ByteArray(16) { n.toByte() }

    /** Two routers bound to each other, the sender's transport ready. */
    private fun pair(config: PeerConfig): Pair<Router, Router> {
        val ecu = Router("ecu", 8u, 50)
        val hmi = Router("hmi", 8u, 50)
        ecu.addPeer("hmi", config)
        hmi.addPeer("ecu", config)
        assertTrue(done(ecu.peerReady("hmi", 0)).isEmpty())
        return ecu to hmi
    }

    private fun done(routed: Routed): List<Effect> = when (routed) {
        is Routed.Done -> routed.effects
        is Routed.Refused -> fail("refused: ${routed.error}")
    }

    private fun transmitted(routed: Routed): ByteArray {
        val effects = done(routed)
        val only = effects.singleOrNull() as? Effect.Transmit ?: fail("expected one transmission, got $effects")
        assertEquals("hmi", only.peer)
        return only.bytes
    }

    private fun decode(bytes: ByteArray): Envelope = Envelope.decode(SceCursor(bytes)) ?: fail("not an envelope")

    private fun events(effects: List<Effect>): List<String> =
        effects.map { (it as? Effect.Deliver)?.event ?: fail("expected only deliveries, got $effects") }

    @Test
    fun aSendToAReadyPeerCrossesAsTheEnginesEventData() {
        val (ecu, hmi) = pair(ordered)
        val bytes = transmitted(ecu.send(request("#hmi", "speed.changed", """{"kph":42}"""), id(1), 0))
        assertEquals(
            listOf(Effect.Deliver("speed.changed", """{"kph":42}""", "ecu", "send.1")),
            done(hmi.receive("ecu", bytes, 0)),
        )
    }

    @Test
    fun aSendWithNoDataTravelsWithNoPayload() {
        val (ecu, hmi) = pair(ordered)
        val bytes = transmitted(ecu.send(request("#hmi", "ping", ""), id(2), 0))
        assertEquals(PayloadCodec.NONE, decode(bytes).datacontenttype)
        assertEquals(listOf(Effect.Deliver("ping", "", "ecu", "send.1")), done(hmi.receive("ecu", bytes, 0)))
    }

    @Test
    fun aSendWithoutAnIdCarriesNoSubject() {
        val (ecu, hmi) = pair(ordered)
        val bytes = transmitted(ecu.send(request("#hmi", "ping", "", sendId = ""), id(3), 0))
        assertNull(decode(bytes).subject)
        assertEquals(listOf(Effect.Deliver("ping", "", "ecu", null)), done(hmi.receive("ecu", bytes, 0)))
    }

    @Test
    fun anOrderedBindingStampsFromOneSoTheReceiverRestoresSendOrder() {
        val (ecu, hmi) = pair(ordered)
        val sent = listOf("a", "b", "c").mapIndexed { i, event ->
            transmitted(ecu.send(request("#hmi", event, ""), id(i + 1), 0))
        }
        assertEquals(listOf(1uL, 2uL, 3uL), sent.map { decode(it).sequence_no })
        assertEquals(listOf("a"), events(done(hmi.receive("ecu", sent[0], 0))))
        assertTrue(done(hmi.receive("ecu", sent[2], 0)).isEmpty())
        assertEquals(listOf("b", "c"), events(done(hmi.receive("ecu", sent[1], 0))))
    }

    @Test
    fun anUnorderedBindingDoesNotStamp() {
        val (ecu, _) = pair(unordered)
        assertNull(decode(transmitted(ecu.send(request("#hmi", "a", ""), id(1), 0))).sequence_no)
    }

    @Test
    fun anUnstampedEnvelopeOnAnOrderedBindingIsMissingItsSequence() {
        val (ecu, _) = pair(unordered)
        val hmi = Router("hmi", 8u, 50)
        hmi.addPeer("ecu", ordered)
        val bytes = transmitted(ecu.send(request("#hmi", "a", ""), id(1), 0))
        assertEquals(
            listOf(Effect.Raise("ecu", Signal.MissingSequence("ecu"))),
            done(hmi.receive("ecu", bytes, 0)),
        )
    }

    @Test
    fun aResentEnvelopeIsDeliveredOnce() {
        val (ecu, hmi) = pair(unordered)
        val bytes = transmitted(ecu.send(request("#hmi", "a", ""), id(7), 0))
        assertEquals(listOf("a"), events(done(hmi.receive("ecu", bytes, 0))))
        assertTrue(done(hmi.receive("ecu", bytes, 1)).isEmpty())
    }

    @Test
    fun aPeerNotYetReadyHoldsTheSendUntilItIs() {
        val ecu = Router("ecu", 8u, 50)
        ecu.addPeer("hmi", ordered)
        assertTrue(done(ecu.send(request("#hmi", "a", ""), id(1), 0)).isEmpty())
        transmitted(ecu.peerReady("hmi", 10))
    }

    @Test
    fun aMeshTargetWithNoBindingIsUnreachableNotUnserved() {
        val ecu = Router("ecu", 8u, 50)
        assertEquals(
            listOf(Effect.Raise("nowhere", Signal.TransportUnavailable)),
            done(ecu.send(request("#nowhere", "a", ""), id(1), 0)),
        )
    }

    @Test
    fun aTargetThatIsNotAMeshPeerIsRefusedToTheHost() {
        val ecu = Router("ecu", 8u, 50)
        for (target in listOf("hmi", "#", "#_internal", "#_parent", "")) {
            assertEquals(
                Routed.Refused(RouterError.NotMeshTarget(target)),
                ecu.send(request(target, "a", ""), id(1), 0),
            )
        }
    }

    @Test
    fun theMeshTargetPredicateIsTheCppCores() {
        // Every row of SendHelper::isMeshTarget's contract, as the Rust core's test states it.
        assertEquals("hmi", meshPeer("#hmi"))
        assertEquals("h", meshPeer("#h"))
        assertEquals("h_1", meshPeer("#h_1"))
        assertNull(meshPeer("#"))
        assertNull(meshPeer("#_internal"))
        assertNull(meshPeer("#_scxml_session"))
        assertNull(meshPeer("hmi"))
        assertNull(meshPeer(""))
    }

    @Test
    fun bytesThatAreNotAnEnvelopeAreReportedAsCorruptCbor() {
        val hmi = Router("hmi", 8u, 50)
        hmi.addPeer("ecu", ordered)
        assertEquals(
            listOf(Effect.Raise("ecu", Signal.EnvelopeCorrupt(null, "cbor"))),
            done(hmi.receive("ecu", byteArrayOf(0xFF.toByte()), 0)),
        )
    }

    @Test
    fun aPayloadTheEngineCannotBeHandedIsCorruptInItsOwnCodec() {
        val hmi = Router("hmi", 8u, 50)
        hmi.addPeer("ecu", unordered)
        val cases = listOf(PayloadCodec.CBOR to "cbor", PayloadCodec.TYPED to "typed", PayloadCodec.RAW to "raw")
        for ((i, case) in cases.withIndex()) {
            val (codec, name) = case
            val bytes = Envelope(
                id = id(i + 1),
                source = "ecu",
                event_type = "a",
                pattern = PatternKind.FIRE_FORGET,
                datacontenttype = codec,
                data = byteArrayOf(1),
            ).encodeToByteArray() ?: fail("encodes")
            assertEquals(
                listOf(Effect.Raise("ecu", Signal.EnvelopeCorrupt("ecu", name))),
                done(hmi.receive("ecu", bytes, 0)),
            )
        }
    }

    @Test
    fun aRaisedRowCarriesTheBindingItWasObservedOn() {
        val ecu = Router("ecu", 8u, 50)
        ecu.addPeer("hmi", ordered)
        val raised = done(ecu.send(request("#nowhere", "a", ""), id(1), 0)).single() as Effect.Raise
        assertEquals(
            """{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"nowhere"}""",
            raised.signal.eventData(ecu.binding(raised.peer)),
        )
        done(ecu.peerReady("hmi", 0))
        val lost = done(ecu.peerNotReady("hmi")).single() as Effect.Raise
        assertEquals(
            """{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}""",
            lost.signal.eventData(ecu.binding(lost.peer)),
        )
    }

    @Test
    fun aPeerTheHostNeverBoundIsTheHostsError() {
        val hmi = Router("hmi", 8u, 50)
        assertEquals(
            Routed.Refused(RouterError.UnknownPeer("ecu")),
            hmi.receive("ecu", byteArrayOf(0xFF.toByte()), 0),
        )
        assertEquals(Routed.Refused(RouterError.UnknownPeer("ecu")), hmi.peerReady("ecu", 0))
    }
}
