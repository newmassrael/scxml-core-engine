// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inbox of a Kotlin engine: the one part of it a thread that does not own it
// may use (docs/adr/0084, ARCHITECTURE.md "Engine Threading").

package com.sce.runtime

/**
 * What may be posted to an engine's [EngineInbox]. The set is closed: it is the
 * inputs that have a caller on another thread, each carrying exactly what the
 * owner-thread operation of the same name takes, because the owner applies a
 * posted input by calling that operation.
 *
 * [E] is the event type of the machine the inbox belongs to; the arms that carry
 * no event of the document are inputs of every machine.
 */
sealed class InboxInput<out E : Event> {
    /**
     * An external event, applied with [StateMachineEngine.send]. [metadata]
     * defaults to what `send(event)` gives an event.
     */
    data class ExternalEvent<out E : Event>(
        val event: E,
        val metadata: EventMetadata = EventMetadata.EMPTY,
    ) : InboxInput<E>()

    /**
     * An external event named by [name], applied with
     * [StateMachineEngine.sendEventByName]: resolved against the document's
     * names when it is applied, and dropped then if the document has no event
     * the name could be delivered as.
     */
    data class ExternalEventByName(
        val name: String,
        val metadata: EventMetadata = EventMetadata.external(),
    ) : InboxInput<Nothing>()

    /**
     * The end of start [token] of the host-run invocation [invokeId] of type
     * [processorType]: a completion (`done.invoke`) when [done], a failure
     * (`error.invoke`) otherwise, with [data] as `_event.data` and [origin] /
     * [originType] as `_event.origin` / `_event.origintype`.
     *
     * [sessionId] `null` ends an invocation of the machine whose inbox this is,
     * applied with [StateMachineEngine.completeHostInvoke] or
     * [StateMachineEngine.failHostInvoke]; a session id ends one of that
     * session of the tree, applied with [StateMachineEngine.completeHostInvokeIn]
     * or [StateMachineEngine.failHostInvokeIn]. The token check those make is
     * made when the input is applied, so an ending posted for a start that was
     * cancelled or started again by then raises nothing, as a direct one would.
     */
    data class HostInvokeEnding(
        val sessionId: String?,
        val processorType: String,
        val invokeId: String,
        val token: Long,
        val done: Boolean,
        val data: String,
        val origin: String = "",
        val originType: String = "",
    ) : InboxInput<Nothing>()

    /**
     * What a Mesh endpoint has for a session, applied with
     * [StateMachineEngine.applyMeshCall]: on the machine whose inbox this is when
     * [sessionId] is `null`, on that session of the tree otherwise, and on none
     * when the tree no longer holds it.
     */
    data class MeshDelivery(
        val call: MeshCall,
        val sessionId: String? = null,
    ) : InboxInput<Nothing>()
}

/**
 * The inbox of one engine ([StateMachineEngine.inbox]): a multi-producer,
 * single-consumer queue, and the only part of an engine that is safe to use from
 * any thread. The engine's owner applies what was posted at its next macrostep
 * boundary, before it takes the next external event, in the order the posts were
 * taken.
 */
interface EngineInbox<E : Event> {
    /**
     * Post [input]. `true` when the inbox took it. The inbox is unbounded, so it
     * takes every post from the moment the engine is built until the engine stops
     * ([StateMachineEngine.stop]) or its run reaches a top-level `<final>`, and
     * refuses every post after either, until the engine is started again. A post
     * taken before the engine starts is applied at its first macrostep boundary;
     * one taken and not yet applied when the engine stops is discarded with its
     * queues.
     */
    fun post(input: InboxInput<E>): Boolean

    /**
     * Install [waker], called on the posting thread after each post the inbox
     * took, so a host that drives the engine itself ([StateMachineEngine.tick])
     * knows there is something to apply. `null` removes it. An engine that runs
     * its own loop ([StateMachineEngine.start]) is woken by the post itself and
     * needs none.
     */
    fun wakeWith(waker: (() -> Unit)?)
}
