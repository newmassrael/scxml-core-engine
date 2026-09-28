// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.generated.rpc_status.RpcStatus
import com.sce.runtime.StateMachineEngine

/** What deployment says about one peer this machine talks to. */
data class PeerConfig(
    /** The binding's transport kind (`"wss"`, `"custom_tcp"`, ...): the `transport` column of its §mesh-16.7 rows. */
    val transport: String,
    /** deploy.yaml's `outbound_buffer:` section, or `null` when the machine declares none and every send is dispatched directly (§mesh-10.10). */
    val buffer: OutboundBuffer?,
    /** deploy.yaml's `retry` block, if the binding has one (§mesh-10.10). */
    val retry: RetryPolicy?,
    /** Whether envelopes TO this peer carry a `sequence_no`: `ordering: required` on a transport that does not order (§mesh-10.6.3). */
    val stampSequence: Boolean,
    /** How envelopes FROM this peer are delivered (§mesh-10.5, §mesh-10.6). */
    val delivery: Delivery,
    /** The machines whose reply may answer a request sent to this peer — the binding's `reply_from:`, or the peer alone (§mesh-14.6). Never empty. */
    val responders: List<String>,
    /** deploy.yaml's binding-level request deadline, which a request to this peer takes when it carries no `_mesh_deadline_ms`; `null` lets it wait for its reply indefinitely (§mesh-9.5). */
    val deadlineMs: Long?,
    /**
     * The events this machine sends to the peer as replies (`service.response.*`, §mesh-8.1). One goes out as
     * `RpcReply` with the invokeid of the request being handled, which is how the requester matches it
     * (§mesh-10.7); every other send is `FireForget`.
     */
    val replyEvents: List<String>,
)

/** One thing the host must do. */
sealed class Effect {
    /** Hand [bytes] to the transport bound to [peer]. */
    class Transmit(val peer: String, val bytes: ByteArray) : Effect()

    /**
     * Raise [event] on the engine's external queue with [data] as
     * `_event.data`; [source] is the machine that sent it, [sendId] the id
     * its `<send>` carried (§mesh-10.7: the envelope's `subject`), and
     * [invokeId] the request's wire invokeid as hex when the envelope is an
     * `RpcRequest` — the one field that comes back on the reply (§mesh-10.7).
     */
    data class Deliver(
        val event: String,
        val data: String,
        val source: String,
        val sendId: String?,
        val invokeId: String?,
    ) : Effect()

    /** Raise `error.communication` with this §mesh-16.7 row; [peer] names the binding it is about, when there is one. */
    data class Raise(val peer: String?, val signal: Signal) : Effect()

    /**
     * End the start [token] of the SCXML invocation [invokeId] with
     * `done.invoke.<invokeId>`: [data] is the reply's payload text and [source]
     * the machine that answered, whose `mesh://<source>` is the event's
     * `_event.origin` (§mesh-9.5).
     */
    data class Complete(val invokeId: String, val token: Long, val data: String, val source: String) : Effect()

    /**
     * End it with `error.invoke.<invokeId>` instead: [data] is the §mesh-10.7.1
     * `errorName: "invoke"` object, and [source] the machine that answered —
     * `null` when the requester's own deadline ended it.
     */
    data class Fail(val invokeId: String, val token: Long, val data: String, val source: String?) : Effect()
}

/** What became of a request [Router.invoke] was handed. */
sealed class Invoked {
    /** It is on its way (or queued for its peer); its answer arrives later as [Effect.Complete] or [Effect.Fail]. */
    class Started(val effects: List<Effect>) : Invoked()

    /**
     * It could not reach the wire, so the invocation never starts: the host
     * refuses it with this `_event.data`, and the engine raises
     * `error.execution` (§mesh-9.5's pre-envelope tier).
     */
    data class Refused(val data: String) : Invoked()

    /** Only the host can act on it (see [RouterError]). */
    data class Failed(val error: RouterError) : Invoked()
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

    /** A Mesh request reached [Router.invoke] without a reserved param the build always gives it. */
    data class MissingParam(val name: String) : RouterError()

    /** A reserved param's value is not what the build writes there. */
    data class BadParam(val name: String, val value: String) : RouterError()
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
        val outbound = Outbound(config.buffer)
        var nextSequence: ULong = 1u
    }

    private val inbound = Inbound(dedupWindow, gapTimeoutMs)
    private val peers = mutableMapOf<String, Peer>()
    private val requests = Correlation()

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
        // §mesh-8.1, §mesh-10.7: a reply the deployment names for this peer goes
        // out as `RpcReply`, carrying the invokeid of the request the document
        // was handling when it sent it — the one field that makes the round
        // trip. A reply sent with no request current carries none, and the
        // requester delivers it as the event it names.
        val reply = request.eventName in peer.config.replyEvents
        val envelope = Envelope(
            id = id,
            source = machine,
            event_type = request.eventName,
            pattern = if (reply) PatternKind.RPC_REPLY else PatternKind.FIRE_FORGET,
            datacontenttype = payloadCodec(request.eventData),
            data = request.eventData.encodeToByteArray(),
            // §mesh-10.7: the receiver's `_event.sendid` is the envelope's `subject`.
            subject = request.sendId.ifEmpty { null },
            invoke_id = if (reply) unhex(request.invokeId) else null,
            rpc_status = if (reply) RpcStatus.OK else null,
        )
        return transmit(peerName, peer, envelope, nowMs)?.let { Routed.Done(it) }
            ?: Routed.Refused(RouterError.Encode)
    }

    /**
     * Start an `<invoke type="sce:mesh-rpc">` the engine handed the host
     * (§mesh-9.5): write its `RpcRequest` and keep it until its reply, its
     * deadline or its cancellation ends it.
     *
     * [wireId] is the `invoke_id` the host minted for this invocation and [id]
     * the envelope's own id — two UUID v7s, so correlation and dedup never
     * share a key. [nowMs] is the host's monotonic clock, which the deadline is
     * kept against, and [nowUnixMs] its wall clock, which the wire's
     * `deadline_unix_ms` is written in.
     *
     * A target that names no Mesh peer, or one this router has no binding for,
     * cannot reach the wire: the invocation is [Invoked.Refused].
     */
    fun invoke(
        request: StateMachineEngine.HostInvokeRequest,
        wireId: ByteArray,
        id: ByteArray,
        nowMs: Long,
        nowUnixMs: Long,
    ): Invoked {
        val peerName = meshPeer(request.src)
            ?: return Invoked.Refused(srcNotFoundData("'${request.src}' names no Mesh peer"))
        val peer = peers[peerName] ?: return Invoked.Refused(srcNotFoundData("no binding for '#$peerName'"))
        val event = request.params[MESH_EVENT_PARAM]?.firstOrNull()
            ?: return Invoked.Failed(RouterError.MissingParam(MESH_EVENT_PARAM))
        // §mesh-9.5 deadline precedence: the invoke's own param, else the binding's, else none.
        val deadlineMs = when (val text = request.params[MESH_DEADLINE_PARAM]?.firstOrNull()) {
            null -> peer.config.deadlineMs
            // Read as the Rust core reads it — any u64 — and held as a Long,
            // beyond which no deadline is distinguishable from none.
            else -> text.toULongOrNull()?.let { minOf(it, Long.MAX_VALUE.toULong()).toLong() }
                ?: return Invoked.Failed(RouterError.BadParam(MESH_DEADLINE_PARAM, text))
        }
        val envelope = Envelope(
            id = id,
            source = machine,
            event_type = event,
            pattern = PatternKind.RPC_REQUEST,
            datacontenttype = payloadCodec(request.eventData),
            data = request.eventData.encodeToByteArray(),
            invoke_id = wireId,
            deadline_unix_ms = deadlineMs?.let { saturatingAdd(nowUnixMs, it).toULong() },
        )
        val effects = transmit(peerName, peer, envelope, nowMs) ?: return Invoked.Failed(RouterError.Encode)
        requests.register(
            wireId,
            Pending(request.invokeId, request.token, peer.config.responders, deadlineMs?.let { saturatingAdd(nowMs, it) }),
        )
        return Invoked.Started(effects)
    }

    /**
     * The state that started the start [token] of [invokeId] exited: stop
     * waiting for its answer. Nothing goes on the wire, and an answer that
     * arrives later is dropped (§mesh-9.5, `<cancel>`).
     */
    fun cancelInvoke(invokeId: String, token: Long) {
        requests.cancel(invokeId, token)
    }

    /**
     * Write [envelope] for [peerName] and hand it to that peer's send half:
     * what [send] and [invoke] share once they know what to say. `null` when
     * it could not be written.
     */
    private fun transmit(peerName: String, peer: Peer, envelope: Envelope, nowMs: Long): List<Effect>? {
        envelope.sequence_no = if (peer.config.stampSequence) peer.nextSequence else null
        val bytes = envelope.encodeToByteArray() ?: return null
        // §mesh-10.6.3: the counter advances once per envelope written, and has
        // no wrap guard.
        if (peer.config.stampSequence) peer.nextSequence += 1u
        return when (val admitted = peer.outbound.admit(bytes, nowMs)) {
            is Admitted.Send -> listOf(Effect.Transmit(peerName, admitted.bytes))
            Admitted.Queued -> emptyList()
            is Admitted.Dropped -> listOf(Effect.Raise(peerName, admitted.signal))
        }
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
        // §mesh-14.6: a reply is checked against its request's responder set on
        // arrival, while the binding it came in on is still known — an ordered
        // binding may hold it and release it on a later tick. One from outside
        // the set is refused before the dedup window or the ordering hold sees
        // it, so it takes no slot in either.
        undeclaredReply(bytes, peer)?.let { return Routed.Done(listOf(Effect.Raise(peer, it))) }
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

    /**
     * End every ordering gap that has waited out its timeout by [nowMs], then
     * every request whose deadline has passed: each ends in `error.invoke` with
     * `deadlineExceeded`, the same shape a peer's `DeadlineExceeded` reply takes
     * (§mesh-10.7.1).
     */
    fun tick(nowMs: Long): Routed {
        val gaps = when (val admission = inbound.tick(nowMs)) {
            is Admission.Admitted -> effectsOf(admission.outcome, null)
            is Admission.Refused -> when (val error = admission.error) {
                is AdmitError.Rule -> return Routed.Refused(RouterError.Rule(error.error))
                // A tick reads no bytes and stamps nothing.
                AdmitError.Malformed, AdmitError.Unstamped -> error("Inbound.tick decodes no envelope")
            }
        }
        val expired = requests.expire(nowMs).map { (wireIdHex, pending) ->
            Effect.Fail(
                pending.invokeId,
                pending.token,
                invokeErrorData(RpcStatus.DEADLINE_EXCEEDED, null, null, wireIdHex),
                null,
            )
        }
        return Routed.Done(gaps + expired)
    }

    /**
     * Row 14 for [bytes], if they are a reply to a live request that arrived on
     * a binding outside its responder set. Bytes that do not decode are left for
     * the receive half to report.
     */
    private fun undeclaredReply(bytes: ByteArray, peer: String): Signal? {
        val envelope = Envelope.decode(SceCursor(bytes)) ?: return null
        val wireId = replyTo(envelope) ?: return null
        return when (requests.check(wireId, peer)) {
            Answer.UNDECLARED -> Signal.RpcReplyFromUndeclaredPeer(envelope.source, hex(wireId))
            Answer.ADMITTED, Answer.UNKNOWN -> null
        }
    }

    /** What the receive half released, as effects, in release order, followed by the rows it observed. */
    private fun effectsOf(outcome: Outcome, peer: String?): List<Effect> =
        outcome.released.mapNotNull { envelope ->
            // A reply's responder was checked on arrival.
            when (val wireId = replyTo(envelope)) {
                null -> deliver(envelope, peer)
                else -> answer(envelope, wireId, peer)
            }
        } + outcome.signals.map { Effect.Raise(peer, it) }

    /**
     * How a released reply ends the request [wireId] (§mesh-9.5), or `null`
     * when no request is waiting on it any more — it was answered, cancelled or
     * expired, and a late answer is dropped.
     *
     * `Ok` — or no status at all, which the requester reads as `Ok` as the C++
     * core does — completes it with the reply's payload; any other status fails
     * it. An `Ok` whose payload the engine cannot be handed is §mesh-16.7 row 4
     * and ends nothing: the request stays waiting for an answer it can use, or
     * for its deadline.
     */
    private fun answer(envelope: Envelope, wireId: ByteArray, peer: String?): Effect? {
        val status = envelope.rpc_status ?: RpcStatus.OK
        if (status == RpcStatus.OK) {
            val data = payloadText(envelope)
            if (data is PayloadText.Unreadable) {
                return if (requests.isWaiting(wireId)) {
                    Effect.Raise(peer, Signal.EnvelopeCorrupt(envelope.source, data.codec))
                } else {
                    null
                }
            }
            val pending = requests.retire(wireId) ?: return null
            return Effect.Complete(pending.invokeId, pending.token, (data as PayloadText.Text).text, envelope.source)
        }
        val pending = requests.retire(wireId) ?: return null
        return Effect.Fail(
            pending.invokeId,
            pending.token,
            invokeErrorData(status, envelope.rpc_error_message, envelope.source, hex(wireId)),
            envelope.source,
        )
    }
}

/**
 * The wire id of the request [envelope] answers, when it is a reply that
 * carries one. A reply to a request this router never sent, or one already
 * retired, is then dropped rather than delivered: this core sends every
 * request with an `invoke_id`, so nothing else can be waiting for it.
 */
private fun replyTo(envelope: Envelope): ByteArray? =
    envelope.invoke_id?.takeIf { envelope.pattern == PatternKind.RPC_REPLY && it.size == 16 }

/**
 * The codec a payload of the sending engine's own `_event.data` text travels
 * in: JSON, or nothing when there is none — the rule the C++ core applies to a
 * child's donedata (ChildSessionAdapter.h).
 */
private fun payloadCodec(eventData: String): PayloadCodec =
    if (eventData.isEmpty()) PayloadCodec.NONE else PayloadCodec.JSON

/** An envelope's payload as the text the engine is handed, or the codec that makes it unreadable. */
private sealed class PayloadText {
    class Text(val text: String) : PayloadText()

    class Unreadable(val codec: String) : PayloadText()
}

/**
 * A JSON or empty payload is the `_event.data` text the sender's engine wrote;
 * a payload in another codec is bytes the engine's text surface cannot be
 * handed, named by that codec so §mesh-16.7 row 4 can say which.
 */
private fun payloadText(envelope: Envelope): PayloadText =
    when (envelope.datacontenttype) {
        PayloadCodec.NONE -> PayloadText.Text("")
        PayloadCodec.JSON -> envelope.data.decodeToStringOrNull()?.let { PayloadText.Text(it) }
            ?: PayloadText.Unreadable("json")
        PayloadCodec.CBOR -> PayloadText.Unreadable("cbor")
        PayloadCodec.TYPED -> PayloadText.Unreadable("typed")
        PayloadCodec.RAW -> PayloadText.Unreadable("raw")
    }

/** [a] + [b] for non-negative [b], held at `Long.MAX_VALUE` rather than wrapping. */
private fun saturatingAdd(a: Long, b: Long): Long = if (a > Long.MAX_VALUE - b) Long.MAX_VALUE else a + b

/**
 * The event an admitted envelope raises. A JSON or empty payload is the
 * `_event.data` text the sender's engine wrote; a payload in another codec is
 * bytes the engine's text surface cannot be handed, so it is §mesh-16.7 row 4
 * naming that codec. [peer] is the binding it arrived on, when a receipt
 * rather than a tick released it.
 */
private fun deliver(envelope: Envelope, peer: String?): Effect =
    when (val data = payloadText(envelope)) {
        is PayloadText.Text -> Effect.Deliver(
            envelope.event_type,
            data.text,
            envelope.source,
            envelope.subject,
            // §mesh-10.7: `_event.invokeid` of an inbound request is its wire invokeid, as hex.
            envelope.invoke_id?.takeIf { envelope.pattern == PatternKind.RPC_REQUEST }?.let(::hex),
        )
        is PayloadText.Unreadable -> Effect.Raise(peer, Signal.EnvelopeCorrupt(envelope.source, data.codec))
    }

/** The bytes as UTF-8 text, or `null` when they are not UTF-8. */
private fun ByteArray.decodeToStringOrNull(): String? =
    try {
        decodeToString(throwOnInvalidSequence = true)
    } catch (_: CharacterCodingException) {
        null
    }
