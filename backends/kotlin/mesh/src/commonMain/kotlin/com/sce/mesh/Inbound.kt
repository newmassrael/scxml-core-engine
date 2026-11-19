// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core's receive half: an envelope off the wire, through duplicate
// suppression (SCE_MESH.md §mesh-10.5) and receiver ordering (§mesh-10.6), to
// the envelopes the engine may now see, in order. The Kotlin twin of
// backends/rust/mesh/src/inbound.rs, with the semantics of the C++
// `OrderingBuffer` and `DedupRouter`.
//
// Every decision is a call into the generated standard rules
// (`sce:std/mesh`); what this adds is the state they are applied to, kept per
// sender, and the envelopes themselves while they are held. The ordering
// rules keep at most 256 held envelopes on the bounded backends and grow on
// the heap backends (stdlib/mesh/ordering_hold.scxml), so this one, a heap
// backend, has no full hold to report.

package com.sce.mesh

import com.sce.forge.runtime.AlgorithmError
import com.sce.forge.runtime.AlgorithmResult
import com.sce.forge.runtime.SceCursor
import com.sce.generated.dedup_admit.dedupAdmit
import com.sce.generated.dedup_holds.dedupHolds
import com.sce.generated.envelope.Envelope
import com.sce.generated.envelope_id.EnvelopeIdPayload
import com.sce.generated.ordering_drain.orderingDrain
import com.sce.generated.ordering_gap_end.orderingGapEnd
import com.sce.generated.ordering_hold.orderingHold
import com.sce.generated.ordering_prune.orderingPrune
import com.sce.generated.ordering_slot.OrderingSlotPayload
import com.sce.generated.pattern_kind.PatternKind

/**
 * What one binding asks of the envelopes it receives: dedup only where the
 * transport does not already suppress duplicates (§mesh-10.5), ordering only
 * where the binding declares it and the transport does not supply it
 * (§mesh-10.6).
 */
data class Delivery(val dedup: Boolean, val ordered: Boolean)

/**
 * Whether an envelope of [pattern] is in its sender's sequence on a binding
 * that orders (§mesh-10.6.3, §mesh-10.6.4): stamped by the sender from the
 * binding's counter, and released by the receiver in that order. The one
 * answer the send half and the receive half both ask, so a number is taken
 * exactly by the envelopes a receiver waits for.
 *
 * A reply (`RpcReply`) and a field notification (`FieldNotify`) find what they
 * answer by its id, not by their place in the stream: they take no number, and
 * the receiver passes them past its ordering buffer (still through the
 * duplicate window), whether or not their sender numbered them. A binding with
 * durable receipt numbers and orders every envelope (SCE_MESH.md section
 * 10.11); this core carries none. The Rust core's `sequenced`.
 */
internal fun sequenced(pattern: PatternKind): Boolean =
    pattern != PatternKind.RPC_REPLY && pattern != PatternKind.FIELD_NOTIFY

/** What one call released, in the order the engine must see it, and what it signalled. */
class Outcome {
    val released: MutableList<Envelope> = mutableListOf()
    val signals: MutableList<Signal> = mutableListOf()
}

/** Why an envelope was not admitted. */
sealed class AdmitError {
    /** The bytes are not an envelope the standard document accepts. */
    data object Malformed : AdmitError()

    /**
     * The binding is ordered, the envelope is one it orders ([sequenced]),
     * and it carries no `sequence_no` (§mesh-10.6.3). Refused before the
     * dedup window sees its id, so a refused envelope does not take a window
     * slot.
     */
    data object Unstamped : AdmitError()

    /** A rule refused its input: a sequence at the `ULong` maximum, or a clock that cannot be subtracted. */
    data class Rule(val error: AlgorithmError) : AdmitError()
}

/** An admission's answer: what it released, or why it was refused. */
sealed class Admission {
    data class Admitted(val outcome: Outcome) : Admission()

    data class Refused(val error: AdmitError) : Admission()
}

/** A rule refused its input; carried out of the call that asked it. */
private class RuleRefusal(val error: AlgorithmError) : RuntimeException(error.contractName)

private fun <T> AlgorithmResult<T>.value(): T =
    when (this) {
        is AlgorithmResult.Ok -> value
        is AlgorithmResult.Failed -> throw RuleRefusal(error)
    }

/** One sender's receive state. */
private class Sender {
    var window: List<EnvelopeIdPayload> = emptyList()

    /**
     * The sequence this sender is expected to send next; `null` until its
     * first ordered envelope, which anchors it (a receiver that joins
     * mid-stream did not miss what was sent before it listened).
     */
    var next: ULong? = null
    var pending: List<OrderingSlotPayload> = emptyList()
    val held: MutableMap<ULong, Envelope> = mutableMapOf()

    /** Release everything held that follows on from [start] without a gap, and expect what comes after it. */
    fun releaseFrom(start: ULong, out: Outcome) {
        val drained = orderingDrain(pending, start).value()
        for (seq in held.keys.filter { it < drained }.sorted()) {
            out.released += held.remove(seq)!!
        }
        pending = orderingPrune(pending, drained)
        next = drained
    }
}

/**
 * The receive half of a router: every sender's window and held envelopes,
 * with deploy.yaml's `window_size` (§mesh-10.5) and `gap_timeout_ms`
 * (§mesh-10.6.1).
 */
class Inbound(private val dedupWindow: UInt, private val gapTimeoutMs: Long) {
    init {
        require(dedupWindow > 0u) { "deploy.yaml's window_size is 0; §mesh-10.5 needs at least one id" }
        require(gapTimeoutMs >= 0) { "deploy.yaml's gap_timeout_ms is negative" }
    }

    private val senders: MutableMap<String, Sender> = mutableMapOf()

    /**
     * How many senders this half keeps state for: one per link it admitted an
     * envelope from (SCE_MESH.md section 10.5).
     */
    fun senderCount(): Int = senders.size

    /**
     * Admit the envelope [bytes] that arrived on the link of [link], from a
     * binding asking [delivery], at [nowMs] on the host's monotonic clock. The
     * window and ordering state it is judged by are [link]'s, whatever
     * `source` it writes (SCE_MESH.md section 10.5): a caller passes only a
     * link it admitted, so the senders kept are as many as those links.
     * §mesh-10.6.4: an ordered envelope below its sender's expected sequence
     * is dropped, one above it is held, and the one it expects is released
     * together with every held envelope that now follows on.
     */
    fun admit(link: String, bytes: ByteArray, delivery: Delivery, nowMs: Long): Admission {
        // Every kind of rejection is one malformed envelope to the receive half.
        val envelope = Envelope.decode(SceCursor(bytes)).valueOr { return Admission.Refused(AdmitError.Malformed) }
        val sequence = envelope.sequence_no
        // §mesh-10.6.4: a reply or a field notification passes the ordering
        // buffer, numbered or not; every other envelope of an ordered binding
        // is placed by its number.
        val ordered = delivery.ordered && sequenced(envelope.pattern)
        if (ordered && sequence == null) {
            return Admission.Refused(AdmitError.Unstamped)
        }
        return try {
            Admission.Admitted(admitDecoded(link, envelope, sequence, delivery.dedup, ordered, nowMs))
        } catch (refusal: RuleRefusal) {
            Admission.Refused(AdmitError.Rule(refusal.error))
        }
    }

    private fun admitDecoded(
        link: String,
        envelope: Envelope,
        sequence: ULong?,
        dedup: Boolean,
        ordered: Boolean,
        nowMs: Long,
    ): Outcome {
        val out = Outcome()
        val sender = senders.getOrPut(link) { Sender() }
        if (dedup) {
            val id = envelopeId(envelope.id.bytes)
            if (dedupHolds(sender.window, id)) {
                return out
            }
            val full = sender.window.size >= dedupWindow.toInt()
            sender.window = dedupAdmit(sender.window, id, dedupWindow).value()
            if (full) {
                out.signals += Signal.DedupWindowOverflow(link, dedupWindow)
            }
        }
        if (!ordered || sequence == null) {
            out.released += envelope
            return out
        }
        val next = sender.next ?: sequence.also { sender.next = it }
        if (sequence < next) {
            return out
        }
        if (sequence > next) {
            sender.pending = orderingHold(sender.pending, sequence, nowMs)
            // A re-delivery of a held sequence keeps its first copy, as
            // ordering_hold keeps its first arrival time.
            sender.held.getOrPut(sequence) { envelope }
            return out
        }
        out.released += envelope
        if (sequence == ULong.MAX_VALUE) {
            throw RuleRefusal(AlgorithmError.Overflow)
        }
        sender.releaseFrom(sequence + 1u, out)
        return out
    }

    /**
     * End every gap that has waited `gap_timeout_ms` by [nowMs]: signal the
     * sequences lost and release what was held behind them. The host calls
     * this on its own schedule, so a gap ends even when nothing else arrives
     * from that sender. Senders are visited in name order, as the Rust core
     * visits them.
     */
    fun tick(nowMs: Long): Admission =
        try {
            val out = Outcome()
            for (source in senders.keys.sorted()) {
                val sender = senders.getValue(source)
                val next = sender.next ?: continue
                val end = orderingGapEnd(sender.pending, next, nowMs, gapTimeoutMs).value()
                if (end == next) continue
                out.signals += Signal.OrderingGap(source, next, end - 1u)
                sender.releaseFrom(end, out)
            }
            Admission.Admitted(out)
        } catch (refusal: RuleRefusal) {
            Admission.Refused(AdmitError.Rule(refusal.error))
        }
}

/**
 * An envelope id as the rules hold it: its first eight bytes and its last
 * eight, big-endian (`sce:std/mesh/envelope_id`). The envelope document fixes
 * `id` at exactly 16 bytes, so a decoded id always splits.
 */
private fun envelopeId(id: ByteArray): EnvelopeIdPayload {
    fun half(from: Int): ULong {
        var v = 0uL
        for (i in from until from + 8) {
            v = (v shl 8) or (id[i].toULong() and 0xFFuL)
        }
        return v
    }
    return EnvelopeIdPayload(hi = half(0), lo = half(8))
}
