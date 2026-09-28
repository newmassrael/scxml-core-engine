// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core: one machine's Mesh endpoint, joining the send half
// ([Outbound]) and the receive half ([Inbound]) to the engine's
// host-processor surface. The Kotlin twin of backends/rust/mesh/src/router.rs.
//
// Sans-IO: every call answers with [Effect]s and the host performs them — it
// owns the transport, the clock and the randomness an envelope id needs, the
// shape SCE_MESH.md §mesh-6.4 gives a transport: the transport moves bytes,
// the core decides.

package com.sce.mesh

import com.sce.forge.runtime.AlgorithmError
import com.sce.forge.runtime.AlgorithmResult
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.runtime.StateMachineEngine

/** What deployment says about one peer this machine talks to. */
data class PeerConfig(
    /** The binding's transport kind (`"wss"`, `"custom_tcp"`, ...): the `transport` column of its §mesh-16.7 rows. */
    val transport: String,
    /** deploy.yaml's `max_pending_per_target` (§mesh-10.10). */
    val maxPending: UInt,
    /** deploy.yaml's `max_age_ms`, 0 for no bound (§mesh-10.10). */
    val maxAgeMs: Long,
    /** deploy.yaml's `retry` block, if the binding has one (§mesh-10.10). */
    val retry: RetryPolicy?,
    /** Whether envelopes TO this peer carry a `sequence_no`: `ordering: required` on a transport that does not order (§mesh-10.6.3). */
    val stampSequence: Boolean,
    /** How envelopes FROM this peer are delivered (§mesh-10.5, §mesh-10.6). */
    val delivery: Delivery,
)

/** One thing the host must do. */
sealed class Effect {
    /** Hand [bytes] to the transport bound to [peer]. */
    class Transmit(val peer: String, val bytes: ByteArray) : Effect()

    /**
     * Raise [event] on the engine's external queue with [data] as
     * `_event.data`; [source] is the machine that sent it and [sendId] the id
     * its `<send>` carried (§mesh-10.7: the envelope's `subject`).
     */
    data class Deliver(val event: String, val data: String, val source: String, val sendId: String?) : Effect()

    /** Raise `error.communication` with this §mesh-16.7 row; [peer] names the binding it is about, when there is one. */
    data class Raise(val peer: String?, val signal: Signal) : Effect()
}

/** Why the router could not do what the host asked. None of these is a §mesh-16.7 row: each is the host's to act on. */
sealed class RouterError {
    /** The host handed [Router.send] a target that is not a Mesh peer reference (see [meshPeer]). */
    data class NotMeshTarget(val target: String) : RouterError()

    /** The host named a peer it never bound with [Router.addPeer]. */
    data class UnknownPeer(val peer: String) : RouterError()

    /** A standard rule refused its input: a sequence at the `ULong` maximum, or a clock that cannot be subtracted. */
    data class Rule(val error: AlgorithmError) : RouterError()

    /** The envelope could not be written within the bounds its document declares. */
    data object Encode : RouterError()
}

/** A router call's answer: the effects to perform, or why there are none. */
sealed class Routed {
    class Done(val effects: List<Effect>) : Routed()

    data class Refused(val error: RouterError) : Routed()
}

/**
 * The peer a `<send target>` names, when it names one — the runtime's
 * predicate, so the router and a generated send site's choice to reach it are
 * one answer rather than two copies of it.
 */
fun meshPeer(target: String): String? = com.sce.runtime.SendHelper.meshPeer(target)

/**
 * One machine's Mesh endpoint, with the receive side's dedup window and gap
 * timeout (deploy.yaml `window_size`, `gap_timeout_ms`).
 */
class Router(val machine: String, dedupWindow: UInt, gapTimeoutMs: Long) {
    private class Peer(val config: PeerConfig) {
        val outbound = Outbound(config.maxPending, config.maxAgeMs)
        var nextSequence: ULong = 1u
    }

    private val inbound = Inbound(dedupWindow, gapTimeoutMs)
    private val peers = mutableMapOf<String, Peer>()

    /** Bind [peer] — the name a document writes as `#peer`. */
    fun addPeer(peer: String, config: PeerConfig) {
        peers[peer] = Peer(config)
    }

    /** The retry policy the host applies when a transmission to [peer] fails. */
    fun retryPolicy(peer: String): RetryPolicy? = peers[peer]?.config?.retry

    /**
     * The binding a raised row names: the peer an [Effect.Raise] carries, with
     * the transport the deployment bound it to. A peer the deployment never
     * bound is named with no transport.
     */
    fun binding(peer: String?): Binding? = peer?.let { Binding(it, peers[it]?.config?.transport) }

    /**
     * Perform a `<send target="#peer">` the engine handed the host. [id] is the
     * envelope id the host drew (a UUID v7), [nowMs] its monotonic clock. A
     * Mesh target this router has no binding for is unreachable, and says so
     * as §mesh-16.7 row 1.
     */
    fun send(request: StateMachineEngine.HostSendRequest, id: ByteArray, nowMs: Long): Routed {
        val peerName = meshPeer(request.target) ?: return Routed.Refused(RouterError.NotMeshTarget(request.target))
        val peer = peers[peerName]
            ?: return Routed.Done(listOf(Effect.Raise(peerName, Signal.TransportUnavailable)))
        val bytes = Envelope(
            id = id,
            source = machine,
            event_type = request.eventName,
            pattern = PatternKind.FIRE_FORGET,
            // The sending engine's own `_event.data` text, as JSON, or nothing
            // — the rule the C++ core applies (ChildSessionAdapter.h).
            datacontenttype = if (request.eventData.isEmpty()) PayloadCodec.NONE else PayloadCodec.JSON,
            data = request.eventData.encodeToByteArray(),
            // §mesh-10.7: the receiver's `_event.sendid` is the envelope's `subject`.
            subject = request.sendId.ifEmpty { null },
            sequence_no = if (peer.config.stampSequence) peer.nextSequence else null,
        ).encodeToByteArray() ?: return Routed.Refused(RouterError.Encode)
        // §mesh-10.6.3: the counter advances once per envelope written, and has
        // no wrap guard.
        if (peer.config.stampSequence) peer.nextSequence += 1u
        val effects = when (val admitted = peer.outbound.admit(bytes, nowMs)) {
            is Admitted.Send -> listOf(Effect.Transmit(peerName, admitted.bytes))
            Admitted.Queued -> emptyList()
            is Admitted.Dropped -> listOf(Effect.Raise(peerName, admitted.signal))
        }
        return Routed.Done(effects)
    }

    /** [peer]'s transport became ready: release what waited for it. */
    fun peerReady(peer: String, nowMs: Long): Routed {
        val bound = peers[peer] ?: return Routed.Refused(RouterError.UnknownPeer(peer))
        return when (val drained = bound.outbound.markReady(nowMs)) {
            is AlgorithmResult.Failed -> Routed.Refused(RouterError.Rule(drained.error))
            is AlgorithmResult.Ok -> Routed.Done(
                drained.value.send.map { Effect.Transmit(peer, it) } +
                    drained.value.signals.map { Effect.Raise(peer, it) },
            )
        }
    }

    /** [peer]'s transport stopped being ready. */
    fun peerNotReady(peer: String): Routed {
        val bound = peers[peer] ?: return Routed.Refused(RouterError.UnknownPeer(peer))
        return Routed.Done(listOfNotNull(bound.outbound.markNotReady()?.let { Effect.Raise(peer, it) }))
    }

    /**
     * Envelope [bytes] a transport received from [peer], at [nowMs]. An
     * envelope that cannot be read is §mesh-16.7 row 4 and one an ordered binding
     * cannot place is row 11, so both are effects; a rule's refusal is a
     * limit of this receiver, which no row names, so it is returned.
     */
    fun receive(peer: String, bytes: ByteArray, nowMs: Long): Routed {
        val delivery = peers[peer]?.config?.delivery ?: return Routed.Refused(RouterError.UnknownPeer(peer))
        return when (val admission = inbound.admit(bytes, delivery, nowMs)) {
            is Admission.Admitted -> Routed.Done(effectsOf(admission.outcome, peer))
            is Admission.Refused -> when (val error = admission.error) {
                // The bytes are the envelope's CBOR (§mesh-7.5); nothing in
                // them can be trusted to name a source.
                AdmitError.Malformed -> Routed.Done(listOf(Effect.Raise(peer, Signal.EnvelopeCorrupt(null, "cbor"))))
                AdmitError.Unstamped -> Routed.Done(listOf(Effect.Raise(peer, Signal.MissingSequence(peer))))
                is AdmitError.Rule -> Routed.Refused(RouterError.Rule(error.error))
            }
        }
    }

    /** End every ordering gap that has waited out its timeout by [nowMs]. */
    fun tick(nowMs: Long): Routed =
        when (val admission = inbound.tick(nowMs)) {
            is Admission.Admitted -> Routed.Done(effectsOf(admission.outcome, null))
            is Admission.Refused -> when (val error = admission.error) {
                is AdmitError.Rule -> Routed.Refused(RouterError.Rule(error.error))
                // A tick reads no bytes and stamps nothing.
                AdmitError.Malformed, AdmitError.Unstamped -> error("Inbound.tick decodes no envelope")
            }
        }
}

/** What the receive half released, as effects, in release order, followed by the rows it observed. */
private fun effectsOf(outcome: Outcome, peer: String?): List<Effect> =
    outcome.released.map { deliver(it, peer) } + outcome.signals.map { Effect.Raise(peer, it) }

/**
 * The event an admitted envelope raises. A JSON or empty payload is the
 * `_event.data` text the sender's engine wrote; a payload in another codec is
 * bytes the engine's text surface cannot be handed, so it is §mesh-16.7 row 4
 * naming that codec. [peer] is the binding it arrived on, when a receipt
 * rather than a tick released it.
 */
private fun deliver(envelope: Envelope, peer: String?): Effect {
    val codec = when (envelope.datacontenttype) {
        PayloadCodec.NONE -> return Effect.Deliver(envelope.event_type, "", envelope.source, envelope.subject)
        PayloadCodec.JSON -> {
            val text = envelope.data.decodeToStringOrNull()
            if (text != null) return Effect.Deliver(envelope.event_type, text, envelope.source, envelope.subject)
            "json"
        }
        PayloadCodec.CBOR -> "cbor"
        PayloadCodec.TYPED -> "typed"
        PayloadCodec.RAW -> "raw"
    }
    return Effect.Raise(peer, Signal.EnvelopeCorrupt(envelope.source, codec))
}

/** The bytes as UTF-8 text, or `null` when they are not UTF-8. */
private fun ByteArray.decodeToStringOrNull(): String? =
    try {
        decodeToString(throwOnInvalidSequence = true)
    } catch (_: CharacterCodingException) {
        null
    }
