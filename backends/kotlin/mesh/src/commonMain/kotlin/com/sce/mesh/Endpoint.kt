// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host half of a Kotlin Mesh endpoint: the [Router] core put to work with
// a transport, a clock and a source of randomness, and joined to an engine.
// The Kotlin twin of backends/rust/mesh/src/endpoint.rs.
//
// The router decides and this performs. A transmission that fails is retried
// on the binding's schedule (§mesh-10.10) or given up as its §mesh-16.7 row; an
// event the router releases, and every row it raises, becomes an
// [EngineEvent] carrying the `_event` fields the C++ core gives the same event
// (sce/include/mesh/MeshDispatch.h), so a document cannot tell which core it
// runs beside; a Mesh request the router ends becomes the call that ends its
// invocation (§mesh-9.5). What the engine must be told is queued rather than
// handed back from the send handler: the engine raises a handler's answer with
// no origin, where the C++ core raises a row as an envelope from its own
// machine, and one queue keeps an invocation's end in order with the events
// around it.
//
// Confined to one thread — the one that steps the engine. The send handler
// already runs there; a transport that receives on its own thread hands the
// bytes to that one (the engine's coroutine scope, a looper) rather than
// calling [Endpoint.receive] itself. Confinement instead of a lock keeps the
// core in common code and its order the engine's order.

package com.sce.mesh

import com.sce.forge.runtime.AlgorithmResult
import com.sce.runtime.EventMetadata
import com.sce.runtime.IoProcessors
import com.sce.runtime.StateMachineEngine

/**
 * The `<send type>` a Mesh send is lowered to — the runtime's constant, so the
 * type this core serves is the one the engine's router door registers.
 */
const val MESH_PROCESSOR_TYPE = com.sce.runtime.MESH_PROCESSOR_TYPE

/** The `<invoke type>` a Mesh request is lowered to, and the `_event.origintype` of the event that ends it (§mesh-9.5). */
const val MESH_RPC_INVOKE_TYPE = com.sce.runtime.MESH_RPC_INVOKE_TYPE

/** `_event.origin` of a Mesh-delivered event is this scheme and the sending machine's name (§mesh-10.7). */
const val MESH_ORIGIN_SCHEME = "mesh://"

/** Why a transport declined a transmission. */
data class TransportFailure(
    /** Whether sending the same bytes again could succeed; a terminal failure is given up at once. */
    val retryable: Boolean,
    /** The transport's own words, carried as the row's `transport_error`. */
    val message: String?,
)

/** Moves envelope bytes to a peer. Bytes that arrive go to [Endpoint.receive]. */
fun interface Transport {
    /** `null` when the bytes were handed on; why not, otherwise. */
    fun transmit(peer: String, bytes: ByteArray): TransportFailure?
}

/** What the core leaves to the host: the time, and the randomness an envelope id and a retry's jitter need. */
interface Environment {
    /** A monotonic clock, in milliseconds. */
    fun nowMs(): Long

    /** The wall clock, in milliseconds since the Unix epoch — what a request's `deadline_unix_ms` is written in (§mesh-9.5). */
    fun nowUnixMs(): Long

    /** A fresh UUID v7 (§mesh-7.5): an envelope's id, or the wire `invoke_id` a request is correlated by. */
    fun envelopeId(): ByteArray

    /** A non-negative number drawn uniformly, for a retry's jitter. */
    fun jitterDraw(): Long
}

/** An event for the engine's external queue, by name, with its `_event`. */
data class EngineEvent(val name: String, val metadata: EventMetadata)

/** One thing the host must tell its engine, in the order the endpoint came to know it. */
sealed class EngineCall {
    /** Raise this event on the external queue. */
    data class Raise(val event: EngineEvent) : EngineCall()

    /**
     * End the start [token] of the Mesh request [invokeId] — with `done.invoke`,
     * or with `error.invoke` when [failed] — carrying [data] as `_event.data`.
     * [source] is the machine that answered, `null` for a deadline the
     * requester reached itself.
     */
    data class EndInvoke(
        val invokeId: String,
        val token: Long,
        val failed: Boolean,
        val data: String,
        val source: String?,
    ) : EngineCall()
}

/** What an [EventAdmission] answers for an event that arrived. */
sealed class Admit {
    /** The event may reach the machine: as it arrived when [data] is `null`, and with [data] in place of what arrived otherwise. */
    data class Allow(val data: String? = null) : Admit()

    /** The event may not reach the machine; [reason] is told to the host. */
    data class Refuse(val reason: String) : Admit()
}

/**
 * What the host asks of an event that arrived before the machine is told of it: the link it came
 * on (which the host authenticated), its name, and its data as the sender wrote it. It is where a
 * rule that needs the event's content is kept, which a machine of the statically typed data model
 * cannot keep for itself — it reads no `_event` but the payload, so it cannot tell who sent an event
 * or check a signature the payload carries. The Kotlin twin of the Rust `Admission` (the name is
 * taken here by the router's own admission of an envelope).
 */
fun interface EventAdmission {
    fun admit(peer: String, event: String, data: String): Admit
}

/** What a [Sealer] answers for an event the machine sent. */
sealed class Sealed {
    /** The event leaves as it is when [data] is `null`, and with [data] in place of what the machine sent otherwise. */
    data class Leave(val data: String? = null) : Sealed()

    /** The event must not leave at all (a write the host cannot sign or seal as the list requires); [reason] is told to the host. */
    data class Withhold(val reason: String) : Sealed()
}

/**
 * What the host writes into an event as it leaves, after the machine sent it and before it is put
 * in an envelope: the sending half of an [EventAdmission]. One it withholds is dropped, told to the
 * host as [RouterError.EventNotSealed], and nothing else is sent in its place.
 */
fun interface Sealer {
    fun seal(target: String, event: String, data: String): Sealed
}

/** One machine's Mesh endpoint on a host. */
class Endpoint(
    private val router: Router,
    private val transport: Transport,
    private val environment: Environment,
) {
    private class Retry(val peer: String, val bytes: ByteArray, val attempts: Attempts, val dueMs: Long)

    private val retries = mutableListOf<Retry>()
    private val toEngine = mutableListOf<EngineCall>()
    private val hostErrors = mutableListOf<RouterError>()
    private val forbidden = mutableMapOf<String, Set<String>>()
    private var admission: EventAdmission? = null
    private var sealer: Sealer? = null

    /**
     * The events the link [peer] may not deliver to the machine: what arrives on it under one of
     * these names is dropped before the machine is told, and the host is told instead
     * ([RouterError.EventForbidden]). An empty collection lifts it.
     *
     * The link is what the host authenticated when it accepted the connection, and the envelope is
     * whatever the peer wrote, its `source` among it: so what a link may say is decided here, where
     * the link is known, and not by a machine that is told who sent an event by the event.
     */
    fun forbidEventsFrom(peer: String, events: Collection<String>) {
        if (events.isEmpty()) forbidden.remove(peer) else forbidden[peer] = events.toSet()
    }

    /**
     * Put [admission] in front of the machine: every event that arrives and is not already forbidden
     * by its link is shown to it, and one it does not admit is dropped and told to the host as
     * [RouterError.EventNotAdmitted]. Replaces the one set before.
     */
    fun setAdmission(admission: EventAdmission) {
        this.admission = admission
    }

    /** Put [sealer] behind the machine: what it sends leaves with the data the sealer answers with. Replaces the one set before. */
    fun setSealer(sealer: Sealer) {
        this.sealer = sealer
    }

    /** Perform a `<send type="sce:mesh">` the engine handed its host. */
    fun send(request: StateMachineEngine.HostSendRequest) {
        val id = environment.envelopeId()
        val leaving = when (val sealed = sealer?.seal(request.target, request.eventName, request.eventData)) {
            null -> request
            is Sealed.Leave -> if (sealed.data != null) request.copy(eventData = sealed.data) else request
            is Sealed.Withhold -> {
                hostErrors += RouterError.EventNotSealed(request.target, request.eventName, sealed.reason)
                return
            }
        }
        apply(router.send(leaving, id, environment.nowMs()))
    }

    /**
     * Start an `<invoke type="sce:mesh-rpc">` the engine handed its host,
     * answering the engine as a host invoker does: with nothing when the
     * request is on its way — its end is queued later — or with a refusal when
     * it cannot reach the wire (§mesh-9.5's pre-envelope tier).
     */
    fun invoke(request: StateMachineEngine.HostInvokeRequest): StateMachineEngine.HostInvokeResponse {
        val wireId = environment.envelopeId()
        val id = environment.envelopeId()
        return when (val invoked = router.invoke(request, wireId, id, environment.nowMs(), environment.nowUnixMs())) {
            is Invoked.Started -> {
                apply(Routed.Done(invoked.effects))
                StateMachineEngine.HostInvokeResponse()
            }
            is Invoked.Refused -> StateMachineEngine.HostInvokeResponse(refusal = invoked.data)
            // Only the host can act on it; the document is told the request
            // could not start rather than left waiting on one that never left.
            is Invoked.Failed -> {
                hostErrors += invoked.error
                StateMachineEngine.HostInvokeResponse(refusal = invoked.error.toString())
            }
        }
    }

    /** The state that started the start [token] of the Mesh request [invokeId] exited. */
    fun cancelInvoke(invokeId: String, token: Long) {
        router.cancelInvoke(invokeId, token)
    }

    /** Envelope [bytes] the transport received from [peer]. */
    fun receive(peer: String, bytes: ByteArray) {
        when (val routed = router.receive(peer, bytes, environment.nowMs())) {
            is Routed.Done -> apply(Routed.Done(screen(peer, routed.effects)))
            is Routed.Refused -> apply(routed)
        }
    }

    /** What a link may deliver, and what the host admits, applied to the events a receive released. */
    private fun screen(peer: String, effects: List<Effect>): List<Effect> {
        val refused = forbidden[peer]
        return effects.mapNotNull { effect ->
            if (effect !is Effect.Deliver) return@mapNotNull effect
            if (refused?.contains(effect.event) == true) {
                hostErrors += RouterError.EventForbidden(peer, effect.event)
                return@mapNotNull null
            }
            when (val verdict = admission?.admit(peer, effect.event, effect.data)) {
                null -> effect
                is Admit.Allow -> if (verdict.data != null) effect.copy(data = verdict.data) else effect
                is Admit.Refuse -> {
                    hostErrors += RouterError.EventNotAdmitted(peer, effect.event, verdict.reason)
                    null
                }
            }
        }
    }

    /** [peer]'s transport became ready. */
    fun peerReady(peer: String) {
        apply(router.peerReady(peer, environment.nowMs()))
    }

    /** [peer]'s transport stopped being ready. */
    fun peerNotReady(peer: String) {
        apply(router.peerNotReady(peer))
    }

    /**
     * End the ordering gaps that have waited out their timeout, and send again
     * what has waited out its backoff. The host calls this on its own
     * schedule; nothing else makes time pass for the core.
     */
    fun tick() {
        val now = environment.nowMs()
        apply(router.tick(now))
        val due = retries.filter { it.dueMs <= now }
        retries.removeAll(due)
        for (retry in due) {
            transmit(retry.peer, retry.bytes, retry.attempts)
        }
    }

    /** What the engine must now be told, in the order it arose. */
    fun takeCalls(): List<EngineCall> = toEngine.toList().also { toEngine.clear() }

    /**
     * What went wrong that only the host can act on. None of these reaches the
     * document (§mesh-16.7, "Scope of synthesis").
     */
    fun takeHostErrors(): List<RouterError> = hostErrors.toList().also { hostErrors.clear() }

    private fun apply(routed: Routed) {
        val effects = when (routed) {
            is Routed.Refused -> {
                hostErrors += routed.error
                return
            }
            is Routed.Done -> routed.effects
        }
        for (effect in effects) {
            when (effect) {
                is Effect.Transmit -> transmit(effect.peer, effect.bytes, Attempts())
                is Effect.Deliver -> toEngine += EngineCall.Raise(
                    EngineEvent(
                        effect.event,
                        // §mesh-10.7: an inbound request's invokeid, which a reply sent while it is handled carries back.
                        meshMetadata(effect.data, effect.source, effect.sendId ?: "")
                            .copy(invokeId = effect.invokeId ?: ""),
                    ),
                )
                is Effect.Raise -> raise(effect.peer, effect.signal)
                is Effect.Complete ->
                    toEngine += EngineCall.EndInvoke(effect.invokeId, effect.token, false, effect.data, effect.source)
                is Effect.Fail ->
                    toEngine += EngineCall.EndInvoke(effect.invokeId, effect.token, true, effect.data, effect.source)
            }
        }
    }

    /**
     * A transmission the transport accepted and then could not complete — an
     * envelope still queued on a link that closed (SCE_MESH.md §mesh-18.3). It
     * is that envelope's first failed send: sent again on the binding's
     * schedule, or given up as its row.
     */
    fun transmitFailed(peer: String, bytes: ByteArray, failure: TransportFailure) {
        afterFailure(peer, bytes, Attempts(), failure)
    }

    /** Transmit [bytes] to [peer]; a failure waits out the binding's backoff or is given up as its row. */
    private fun transmit(peer: String, bytes: ByteArray, attempts: Attempts) {
        val failure = transport.transmit(peer, bytes) ?: return
        afterFailure(peer, bytes, attempts, failure)
    }

    private fun afterFailure(peer: String, bytes: ByteArray, attempts: Attempts, failure: TransportFailure) {
        val decided = attempts.afterFailure(
            router.retryPolicy(peer),
            failure.retryable,
            failure.message,
            environment.jitterDraw(),
        )
        when (decided) {
            is AlgorithmResult.Failed -> hostErrors += RouterError.Rule(decided.error)
            is AlgorithmResult.Ok -> when (val after = decided.value) {
                is AfterFailure.RetryIn -> retries += Retry(peer, bytes, attempts, environment.nowMs() + after.waitMs)
                is AfterFailure.GiveUp -> raise(peer, after.signal)
            }
        }
    }

    /** Queue the `error.communication` a row raises, as the C++ core raises it: an envelope from its own machine. */
    private fun raise(peer: String?, signal: Signal) {
        toEngine += EngineCall.Raise(
            EngineEvent(
                "error.communication",
                meshMetadata(signal.eventData(router.binding(peer)), router.machine, ""),
            ),
        )
    }
}

/**
 * The `_event` of an event that arrived over Mesh from [source], as §mesh-10.7
 * tabulates it and the C++ core fills it: external, origin `mesh://<source>`,
 * the SCXML processor as origin type, and the send id the envelope carried.
 */
private fun meshMetadata(data: String, source: String, sendId: String): EventMetadata =
    EventMetadata(
        data = data,
        type = "external",
        sendId = sendId,
        origin = MESH_ORIGIN_SCHEME + source,
        originType = IoProcessors.SCXML_PROCESSOR,
    )

/**
 * Tell [engine] what [calls] say, in order. An event name the machine does not
 * declare is dropped, as the engine drops any such event; an invocation's end
 * the engine no longer waits for — its state exited, or it was started again —
 * is refused by the engine's own token check.
 *
 * An answer carries `_event.origin` `mesh://<source>` and origin type
 * `sce:mesh-rpc` (§mesh-9.5); a deadline the requester reached itself has no
 * peer to name, and carries neither.
 */
fun applyTo(engine: StateMachineEngine<*, *>, calls: List<EngineCall>) {
    for (call in calls) {
        when (call) {
            is EngineCall.Raise -> engine.sendEventByName(call.event.name, call.event.metadata)
            is EngineCall.EndInvoke -> {
                val origin = call.source?.let { MESH_ORIGIN_SCHEME + it } ?: ""
                val originType = if (call.source != null) MESH_RPC_INVOKE_TYPE else ""
                if (call.failed) {
                    engine.failHostInvoke(MESH_RPC_INVOKE_TYPE, call.invokeId, call.token, call.data, origin, originType)
                } else {
                    engine.completeHostInvoke(MESH_RPC_INVOKE_TYPE, call.invokeId, call.token, call.data, origin, originType)
                }
            }
        }
    }
}

/**
 * Serve `<send type="sce:mesh">` and `<invoke type="sce:mesh-rpc">` on
 * [engine] with [endpoint]. The send handler answers the engine with nothing,
 * and a request that starts is answered with nothing too: what either produces
 * for the document is queued on the endpoint, and the host applies it with
 * [applyTo] after the step that sent it.
 */
fun register(engine: StateMachineEngine<*, *>, endpoint: Endpoint) {
    engine.registerMeshRouter { request ->
        endpoint.send(request)
        emptyList()
    }
    engine.registerMeshRpcInvoker { event ->
        event.start?.let { return@registerMeshRpcInvoker endpoint.invoke(it) }
        event.cancel?.let { endpoint.cancelInvoke(it.invokeId, it.token) }
        null
    }
}
