// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the router core tells the engine besides the envelopes themselves:
// the `error.communication` rows of SCE_MESH.md §mesh-16.7 that the core,
// rather than a transport, is in a position to observe. The Kotlin twin of
// backends/rust/mesh/src/signal.rs, row for row and byte for byte.

package com.sce.mesh

/** One §mesh-16.7 row, with the extras the row carries. */
sealed class Signal {
    /** ORDERING_GAP (row 12): [source]'s sequences [lostLo]..[lostHi] waited out the gap timeout. */
    data class OrderingGap(val source: String, val lostLo: ULong, val lostHi: ULong) : Signal()

    /** DEDUP_WINDOW_OVERFLOW (row 7): a novel id was admitted to a full window of [windowSize] ids. */
    data class DedupWindowOverflow(val source: String, val windowSize: UInt) : Signal()

    /**
     * ENVELOPE_CORRUPT (row 4): what arrived could not be read. [codec] is
     * `"cbor"` when the bytes are not an envelope the standard document
     * accepts, or the payload's own codec when the envelope decoded but its
     * payload is not one the engine can be handed — then the envelope also
     * named its [source].
     */
    data class EnvelopeCorrupt(val source: String?, val codec: String) : Signal()

    /** MISSING_SEQUENCE (row 11): an envelope reached an ordered binding without a `sequence_no`. */
    data class MissingSequence(val source: String) : Signal()

    /** BACKPRESSURE_DROP (row 9): the target's queue already held [depth] envelopes. */
    data class BackpressureDrop(val depth: UInt) : Signal()

    /** OUTBOUND_STALE_DROP (row 15): a queued envelope had waited [ageMs], past [maxAgeMs]. */
    data class OutboundStaleDrop(val ageMs: Long, val maxAgeMs: Long) : Signal()

    /** TRANSPORT_UNAVAILABLE (row 1). */
    data object TransportUnavailable : Signal()

    /** SEND_FAILED (row 2): declined with no retry configured; [transportError] is the transport's words. */
    data class SendFailed(val transportError: String?) : Signal()

    /** DELIVERY_EXHAUSTED (row 3): given up after [attempts] sends. */
    data class DeliveryExhausted(val attempts: UInt, val transportError: String?) : Signal()

    /**
     * RPC_REPLY_FROM_UNDECLARED_PEER (row 14): a reply from [source] named the
     * live request [invokeId] (the wire id, hex) but arrived on a binding
     * outside that request's responder set (§mesh-14.6). The request stays
     * answerable.
     */
    data class RpcReplyFromUndeclaredPeer(val source: String, val invokeId: String) : Signal()

    /**
     * INVOKE_CHILD_LOST (row 5): the link to [target] was lost while the
     * request [invokeId] (the wire id, hex) waited on it. The request is
     * forgotten, as a cancel forgets it (§mesh-9.5), since no reply can arrive
     * on a link that is gone.
     */
    data class InvokeChildLost(val invokeId: String, val target: String) : Signal()

    /** The row's `reason`, as §mesh-16.7 spells it. */
    val reason: String
        get() = when (this) {
            is OrderingGap -> "ORDERING_GAP"
            is DedupWindowOverflow -> "DEDUP_WINDOW_OVERFLOW"
            is EnvelopeCorrupt -> "ENVELOPE_CORRUPT"
            is MissingSequence -> "MISSING_SEQUENCE"
            is BackpressureDrop -> "BACKPRESSURE_DROP"
            is OutboundStaleDrop -> "OUTBOUND_STALE_DROP"
            TransportUnavailable -> "TRANSPORT_UNAVAILABLE"
            is SendFailed -> "SEND_FAILED"
            is DeliveryExhausted -> "DELIVERY_EXHAUSTED"
            is RpcReplyFromUndeclaredPeer -> "RPC_REPLY_FROM_UNDECLARED_PEER"
            is InvokeChildLost -> "INVOKE_CHILD_LOST"
        }

    /**
     * The `_event.data` of the `error.communication` this row raises: byte for
     * byte what the C++ core's `CommunicationError::toJsonBytes` writes for the
     * same row — `errorName`, `reason`, then each present field in that
     * header's declaration order, absent ones omitted. [binding] supplies
     * `target` and `transport` to the rows whose columns name them.
     */
    fun eventData(binding: Binding?): String {
        val json = ErrorData("communication", reason)
        fun text(key: String, value: String) = json.text(key, value)
        fun number(key: String, value: Any) = json.number(key, value)
        val source = when (this) {
            is OrderingGap -> source
            is DedupWindowOverflow -> source
            is MissingSequence -> source
            is EnvelopeCorrupt -> source
            is RpcReplyFromUndeclaredPeer -> source
            else -> null
        }
        val sending = this is BackpressureDrop || this is OutboundStaleDrop || this == TransportUnavailable ||
            this is SendFailed || this is DeliveryExhausted
        val transportRow = sending || this is EnvelopeCorrupt
        source?.let { text("source", it) }
        if (binding != null) {
            if (sending) text("target", binding.peer)
            if (transportRow) binding.transport?.let { text("transport", it) }
        }
        when (this) {
            is SendFailed -> transportError?.let { text("transport_error", it) }
            is DeliveryExhausted -> {
                transportError?.let { text("transport_error", it) }
                number("attempts", attempts)
            }
            is EnvelopeCorrupt -> text("codec", codec)
            is BackpressureDrop -> number("queue_depth", depth)
            is DedupWindowOverflow -> number("window_size", windowSize)
            is OutboundStaleDrop -> {
                number("age_ms", ageMs)
                number("max_age_ms", maxAgeMs)
            }
            is OrderingGap -> {
                number("lost_seq_lo", lostLo)
                number("lost_seq_hi", lostHi)
            }
            // §mesh-16.7 row 14's columns: `source`, then the request it tried to retire.
            is RpcReplyFromUndeclaredPeer -> text("invoke_id", invokeId)
            // Row 5 names its own `target` column, after `invoke_id`, where the C++ core's field order puts it.
            is InvokeChildLost -> {
                text("invoke_id", invokeId)
                text("target", target)
            }
            is MissingSequence, TransportUnavailable -> Unit
        }
        return json.finish()
    }
}

/**
 * The deployment binding a row is observed on: the peer it names and the
 * transport kind that carries it. A send to a peer the deployment never bound
 * names the peer and no transport.
 */
data class Binding(val peer: String, val transport: String?)
