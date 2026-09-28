// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core, case for case with backends/rust/mesh/src/router.rs.

package com.sce.mesh

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.generated.rpc_status.RpcStatus
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
        buffer = OutboundBuffer(maxPending = 4u, maxAgeMs = 0),
        retry = null,
        stampSequence = true,
        delivery = Delivery(dedup = true, ordered = true),
        // The peer these tests bind is `hmi`; `pair` gives each side its own.
        responders = listOf("hmi"),
        deadlineMs = null,
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
        ecu.addPeer("hmi", config.copy(responders = listOf("hmi")))
        hmi.addPeer("ecu", config.copy(responders = listOf("ecu")))
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
    fun withoutABufferASendIsTransmittedBeforeThePeerIsReady() {
        val ecu = Router("ecu", 8u, 50)
        ecu.addPeer("hmi", unordered.copy(buffer = null))
        transmitted(ecu.send(request("#hmi", "ping", ""), id(7), 0))
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

    // ── §mesh-9.5: `<invoke type="sce:mesh-rpc">`, the requester's half ──

    private val wire = ByteArray(16) { 9 }
    private val wireHex = "09090909090909090909090909090909"

    /** `ecu` bound to `hmi` with a binding-level deadline, and to `mallory`, which is not in `hmi`'s responder set. */
    private fun requester(deadlineMs: Long?): Router {
        val ecu = Router("ecu", 8u, 50)
        val config = unordered.copy(buffer = null, responders = listOf("hmi"), deadlineMs = deadlineMs)
        ecu.addPeer("hmi", config)
        ecu.addPeer("mallory", config.copy(responders = listOf("mallory")))
        return ecu
    }

    private fun ask(src: String, deadlineMs: String?) = StateMachineEngine.HostInvokeRequest(
        processorType = "sce:mesh-rpc",
        invokeId = "ask",
        src = src,
        params = buildMap {
            put(MESH_EVENT_PARAM, listOf("service.request.force"))
            deadlineMs?.let { put(MESH_DEADLINE_PARAM, listOf(it)) }
        },
        eventData = """{"n":3}""",
        token = 7,
    )

    /** Start `ask("#hmi")` at monotonic 10, wall 1000, returning the request envelope's bytes. */
    private fun started(ecu: Router, deadlineMs: String?): ByteArray =
        when (val invoked = ecu.invoke(ask("#hmi", deadlineMs), wire, id(1), 10, 1000)) {
            is Invoked.Started -> transmitted(Routed.Done(invoked.effects))
            else -> fail("expected the request to start, got $invoked")
        }

    private fun reply(n: Int, status: RpcStatus?, message: String?, data: String): ByteArray =
        Envelope(
            id = id(n),
            source = "hmi",
            event_type = "service.response.force",
            pattern = PatternKind.RPC_REPLY,
            datacontenttype = if (data.isEmpty()) PayloadCodec.NONE else PayloadCodec.JSON,
            data = data.encodeToByteArray(),
            invoke_id = wire,
            rpc_status = status,
            rpc_error_message = message,
        ).encodeToByteArray() ?: fail("reply did not encode")

    @Test
    fun aRequestCarriesItsEventPayloadAndWireIds() {
        val envelope = decode(started(requester(500), null))
        assertEquals(PatternKind.RPC_REQUEST, envelope.pattern)
        assertEquals("service.request.force", envelope.event_type)
        assertEquals("""{"n":3}""", envelope.data.decodeToString())
        assertEquals(PayloadCodec.JSON, envelope.datacontenttype)
        assertTrue(wire.contentEquals(envelope.invoke_id))
        assertTrue(id(1).contentEquals(envelope.id))
        // §mesh-9.5 precedence: the binding's deadline when the invoke gives none.
        assertEquals(1500uL, envelope.deadline_unix_ms)
    }

    @Test
    fun theInvokesOwnDeadlineWinsOverTheBindings() {
        assertEquals(1200uL, decode(started(requester(500), "200")).deadline_unix_ms)
    }

    @Test
    fun aRequestWithNoDeadlineCarriesNone() {
        val ecu = requester(null)
        assertNull(decode(started(ecu, null)).deadline_unix_ms)
        assertTrue(done(ecu.tick(Long.MAX_VALUE)).isEmpty(), "no deadline, no expiry")
    }

    @Test
    fun anOkReplyCompletesTheInvocationOnce() {
        val ecu = requester(null)
        started(ecu, null)
        assertEquals(
            listOf(Effect.Complete("ask", 7, """{"force":12}""", "hmi")),
            done(ecu.receive("hmi", reply(2, RpcStatus.OK, null, """{"force":12}"""), 20)),
        )
        // A second answer finds nothing waiting.
        assertTrue(done(ecu.receive("hmi", reply(3, RpcStatus.OK, null, "1"), 21)).isEmpty())
    }

    /** The key is optional on the wire; the requester reads its absence as `Ok`, as the C++ core does. */
    @Test
    fun aReplyWithNoStatusIsOk() {
        val ecu = requester(null)
        started(ecu, null)
        assertEquals(listOf(Effect.Complete("ask", 7, "", "hmi")), done(ecu.receive("hmi", reply(2, null, null, ""), 20)))
    }

    @Test
    fun aFailedReplyFailsTheInvocationWithTheStatusByName() {
        val ecu = requester(null)
        started(ecu, null)
        assertEquals(
            listOf(
                Effect.Fail(
                    "ask",
                    7,
                    """{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"hmi","invoke_id":"$wireHex"}""",
                    "hmi",
                ),
            ),
            done(ecu.receive("hmi", reply(2, RpcStatus.UNAVAILABLE, "busy", ""), 20)),
        )
    }

    /** §mesh-14.6: a reply from outside the responder set is row 14 and leaves the request answerable. */
    @Test
    fun aReplyFromAnUndeclaredPeerIsRefusedAndTheRequestStays() {
        val ecu = requester(null)
        started(ecu, null)
        assertEquals(
            // The envelope's word, reported as written; what refused it was the binding it came in on.
            listOf(Effect.Raise("mallory", Signal.RpcReplyFromUndeclaredPeer("hmi", wireHex))),
            done(ecu.receive("mallory", reply(2, RpcStatus.OK, null, "1"), 20)),
        )
        assertEquals(
            listOf(Effect.Complete("ask", 7, "2", "hmi")),
            done(ecu.receive("hmi", reply(3, RpcStatus.OK, null, "2"), 21)),
        )
    }

    /** §mesh-9.5 `<cancel>`: nothing on the wire, and a later answer is dropped. */
    @Test
    fun aCancelledRequestDropsItsAnswer() {
        val ecu = requester(100)
        started(ecu, null)
        ecu.cancelInvoke("ask", 7)
        assertTrue(done(ecu.receive("hmi", reply(2, RpcStatus.OK, null, "1"), 20)).isEmpty())
        assertTrue(done(ecu.tick(1000)).isEmpty(), "a cancelled request has no deadline")
    }

    @Test
    fun aDeadlineFailsTheInvocationAsDeadlineExceededWithNoSource() {
        val ecu = requester(100)
        started(ecu, null)
        // Kept against the monotonic clock the request started at (10).
        assertTrue(done(ecu.tick(109)).isEmpty())
        assertEquals(
            listOf(
                Effect.Fail(
                    "ask",
                    7,
                    """{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"$wireHex"}""",
                    null,
                ),
            ),
            done(ecu.tick(110)),
        )
        assertTrue(done(ecu.receive("hmi", reply(2, RpcStatus.OK, null, "1"), 120)).isEmpty())
    }

    /** An `Ok` the engine cannot be handed is row 4 and ends nothing: the request still waits for an answer it can use. */
    @Test
    fun anUnreadableOkReplyLeavesTheRequestWaiting() {
        val ecu = requester(null)
        started(ecu, null)
        val raw = Envelope(
            id = id(2),
            source = "hmi",
            event_type = "service.response.force",
            pattern = PatternKind.RPC_REPLY,
            datacontenttype = PayloadCodec.RAW,
            data = byteArrayOf(0xFF.toByte()),
            invoke_id = wire,
        ).encodeToByteArray() ?: fail("reply did not encode")
        assertEquals(
            listOf(Effect.Raise("hmi", Signal.EnvelopeCorrupt("hmi", "raw"))),
            done(ecu.receive("hmi", raw, 20)),
        )
        assertEquals(
            listOf(Effect.Complete("ask", 7, "1", "hmi")),
            done(ecu.receive("hmi", reply(3, RpcStatus.OK, null, "1"), 21)),
        )
    }

    /** §mesh-9.5's pre-envelope tier: a target that cannot reach the wire refuses the invocation. */
    @Test
    fun aRequestThatCannotReachTheWireIsRefused() {
        val ecu = requester(null)
        assertEquals(
            Invoked.Refused("""{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#nobody'"}"""),
            ecu.invoke(ask("#nobody", null), wire, id(1), 0, 0),
        )
        val refused = ecu.invoke(ask("hmi", null), wire, id(1), 0, 0)
        assertTrue(refused is Invoked.Refused && refused.data.contains("names no Mesh peer"), "$refused")
    }

    @Test
    fun aRequestWithoutItsEventParamIsTheHostsError() {
        val request = ask("#hmi", null).let { it.copy(params = it.params - MESH_EVENT_PARAM) }
        assertEquals(
            Invoked.Failed(RouterError.MissingParam(MESH_EVENT_PARAM)),
            requester(null).invoke(request, wire, id(1), 0, 0),
        )
    }
}
