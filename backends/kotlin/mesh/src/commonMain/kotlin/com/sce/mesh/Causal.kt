// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The causal route's delivery layer (docs/adr/0085, sections 3, 3.2, 3.3 and
// 4.2): one copy's side of a causal broadcast group. The Kotlin twin of
// backends/rust/mesh/src/causal.rs.
//
// A change this copy makes is offered here and answered with a verdict:
// [Verdict.Accepted] with the dot the layer tagged it with, or
// [Verdict.Refused] when the log has no room for one more change not yet
// stable. An accepted change is released to the peers only once the host
// reports its journal record committed ([CausalLayer.committed]), so a number
// no peer has seen is the only one a crash can lose. A change that arrives is
// delivered once, and only after every change in its context; one that is not
// yet deliverable is held, and a gap is repaired by exchanging vectors, never
// skipped: there is no gap timeout. Every change sent or delivered is kept
// until the stable cut passes it.
//
// Every decision is a call into the generated standard rules
// (`sce:std/mesh`): whether a change is dropped, delivered or held
// (`causal_admit`), the merge of two vectors (`causal_raise`), whether a vector
// lacks a change (`causal_lacks`) and the stable cut (`causal_cut`). What this
// file adds is the state those rules are applied to and the messages
// themselves while they are held or logged. The layer is generic over the
// message, which the router instantiates with an envelope's bytes, and
// transport-free, like the rest of the core.
//
// The author of a dot is a replica: one incarnation of one member, a UUID v7
// the host draws when an incarnation begins and keeps in the copy's journal
// beside the highest number it used ([CausalStart]). A copy that cannot know
// its highest number begins a new incarnation, never one asked of its peers.

package com.sce.mesh

import com.sce.forge.runtime.AlgorithmError
import com.sce.forge.runtime.AlgorithmResult
import com.sce.generated.causal_admission.CausalAdmission
import com.sce.generated.causal_admit.causalAdmit
import com.sce.generated.causal_cut.causalCut
import com.sce.generated.causal_entry.CausalEntryPayload
import com.sce.generated.causal_known.CausalKnownPayload
import com.sce.generated.causal_lacks.causalLacks
import com.sce.generated.causal_raise.causalRaise
import com.sce.generated.envelope.Envelope

/**
 * The most entries a vector holds: the bound of a view, of the envelope's
 * `causal_vector` and of the rules' lists.
 */
const val CAUSAL_VECTOR_BOUND: Int = 64

/**
 * A replica identity: one incarnation of one member of a group, a 16-byte
 * UUID v7 unique within the group and never used again, held as the rules
 * hold it (`sce:std/mesh/causal_entry`): its first eight bytes ([hi]) and its
 * last eight ([lo]), big-endian. Two replicas order as their bytes do.
 */
data class Replica(val hi: ULong, val lo: ULong) : Comparable<Replica> {
    /** This replica's entry at [seq]. */
    fun entry(seq: ULong): CausalEntryPayload = CausalEntryPayload(hi = hi, lo = lo, seq = seq)

    /** The 16 bytes of the identity. */
    fun toBytes(): ByteArray = ByteArray(16) { i ->
        val half = if (i < 8) hi else lo
        (half shr (8 * (7 - i % 8))).toByte()
    }

    override fun compareTo(other: Replica): Int {
        val byHi = hi.compareTo(other.hi)
        return if (byHi != 0) byHi else lo.compareTo(other.lo)
    }

    companion object {
        /** The replica an entry names. */
        fun of(entry: CausalEntryPayload): Replica = Replica(entry.hi, entry.lo)

        /** The replica whose identity is the 16 bytes [id]. */
        fun fromBytes(id: ByteArray): Replica {
            require(id.size == 16) { "a replica identity is 16 bytes, not ${id.size}" }
            fun half(from: Int): ULong {
                var v = 0uL
                for (i in from until from + 8) {
                    v = (v shl 8) or (id[i].toULong() and 0xFFuL)
                }
                return v
            }
            return Replica(half(0), half(8))
        }
    }
}

/** The dot a change is tagged with: its author and its number, from 1 with no gap. */
data class Dot(val replica: Replica, val seq: ULong) {
    /** This dot as an entry of a vector. */
    fun entry(): CausalEntryPayload = replica.entry(seq)

    companion object {
        /** The dot an entry names. */
        fun of(entry: CausalEntryPayload): Dot = Dot(Replica.of(entry), entry.seq)
    }
}

/**
 * A causal binding's group as the deployment gives it: its [name] and the
 * [view] of epoch 0, the only view until view changes land (docs/adr/0085,
 * section 3.3). Each entry of [view] names one instance as
 * `<device>/<machine>`.
 */
data class CausalGroup(val name: String, val epoch: ULong, val view: List<String>)

/**
 * What a tagged envelope or a `causalVector` carries: the view's [epoch]
 * (envelope key 19) and a [vector] (key 20) whose first entry is its sender's
 * own. On a change that entry is the change's dot and the rest its context; on
 * a `causalVector` the vector is the sender's delivered vector.
 */
data class CausalTag(val epoch: ULong, val vector: List<CausalEntryPayload>)

/**
 * What the host reads from the copy's own journal when a copy starts: the
 * [replica] identity of its incarnation and its [delivered] vector, its own
 * entry being the highest number it used.
 */
data class CausalJournalRecord(val replica: Replica, val delivered: List<CausalEntryPayload>)

/** How an incarnation starts (docs/adr/0085, section 3.3.2). */
sealed class CausalStart {
    /**
     * A normal restart: the copy's journal says who it is and what it
     * delivered, and it continues after its highest number.
     */
    data class Journal(val record: CausalJournalRecord) : CausalStart()

    /**
     * A copy that cannot know its own highest number begins a new
     * incarnation, with an identity the host has just drawn.
     */
    data class NewIncarnation(val replica: Replica) : CausalStart()

    companion object {
        /**
         * The start of a copy whose journal gave [journal], or none: a copy
         * restored from an older save, or set up again from its seed, has no
         * journal record of its own, and begins under [fresh] instead. It never
         * asks its peers for its number: the change with the highest one may be
         * held only by a member that is offline.
         */
        fun fromJournal(journal: CausalJournalRecord?, fresh: Replica): CausalStart =
            if (journal != null) Journal(journal) else NewIncarnation(fresh)
    }
}

/** The layer's answer to a change its copy offers (docs/adr/0093, section 3.1). */
sealed class Verdict {
    /** Tagged with this dot; the host records it in the journal, then calls [CausalLayer.committed]. */
    data class Accepted(val dot: Dot) : Verdict()

    /** Not tagged: it took no number, and no copy takes it in. */
    data class Refused(val refusal: Refusal) : Verdict()
}

/** Why an offer was refused. */
enum class Refusal {
    /**
     * The log holds as many changes not yet stable as the deployment allows;
     * the list tells it as `noRoomForHistory`, and stability frees room.
     */
    NO_ROOM_FOR_HISTORY,

    /** The incarnation used every number a dot can carry. */
    NUMBERS_EXHAUSTED,
}

/** What became of a change offered to [CausalLayer.offer]. */
sealed class Offered {
    /** The layer answered with [verdict]. */
    data class Answered(val verdict: Verdict) : Offered()

    /** The message for the tagged change could not be made; no number was taken. */
    data object Unbuilt : Offered()
}

/** Why something that arrived was not taken in. */
sealed class Rejection {
    /** Tagged with another epoch than the current view's: raised, not delivered. */
    data class OtherEpoch(val epoch: ULong) : Rejection()

    /** Tagged by a replica the view does not hold: raised, not delivered. */
    data class NotInView(val replica: Replica) : Rejection()

    /** A vector with no sender entry, or more entries than a view holds. */
    data object Malformed : Rejection()

    /** A rule refused its input. */
    data class Rule(val error: AlgorithmError) : Rejection()
}

/** What the layer made of a change or a vector that arrived. */
sealed class CausalReceived<out M> {
    /** Taken in: the [messages] it lets this copy deliver, or sends back, in order. */
    data class Messages<out M>(val messages: List<M>) : CausalReceived<M>()

    /** Not taken in, for [rejection]. */
    data class Rejected(val rejection: Rejection) : CausalReceived<Nothing>()
}

/** A change in the log: its dot, the message as it was sent, and whether it has been released to the peers. */
private class CausalLogged<T>(val dot: CausalEntryPayload, val message: T, var released: Boolean)

/** A change held until it is deliverable: its tag and its message. */
private class CausalHeld<T>(val dot: CausalEntryPayload, val context: List<CausalEntryPayload>, val message: T)

/**
 * One copy's causal delivery layer for one [group], started as [start] says,
 * whose log holds at most [room] changes not yet stable.
 */
class CausalLayer<M : Any>(private val group: CausalGroup, start: CausalStart, private val room: Int) {
    /** The replica identity this incarnation tags its changes with. */
    val replica: Replica

    /** What this copy delivered, its own entry among them. */
    var delivered: List<CausalEntryPayload>
        private set

    /** The latest delivered vector known of each other member, by replica. */
    private val known = mutableMapOf<Replica, List<CausalEntryPayload>>()
    private val held = mutableListOf<CausalHeld<M>>()
    private val log = mutableListOf<CausalLogged<M>>()

    init {
        val (id, recorded) = when (start) {
            is CausalStart.Journal -> start.record.replica to start.record.delivered
            is CausalStart.NewIncarnation -> start.replica to emptyList<CausalEntryPayload>()
        }
        replica = id
        // The copy's own entry is always present, at 0 before its first
        // change, so its vector says who sent it.
        delivered = if (recorded.any { Replica.of(it) == id }) recorded else listOf(id.entry(0uL)) + recorded
    }

    /** How many changes are held until they are deliverable. */
    val heldLen: Int get() = held.size

    /** How many changes the log keeps until the stable cut passes them. */
    val logLen: Int get() = log.size

    /** The highest number this incarnation used. */
    val ownSeq: ULong get() = seen(delivered, replica)

    /**
     * Offer a change this copy made. When the log has room, the layer tags it
     * with the next dot and its delivered vector, asks [build] for the message
     * that carries that tag, counts the change delivered here and logs it; the
     * message is released by [committed]. When the log has no room the change
     * is refused and takes no number, and when [build] answers `null` nothing
     * is taken either.
     */
    fun offer(build: (CausalTag) -> M?): Offered {
        // §mesh-10.12, offer and verdict: a log with no room for one more
        // change not yet stable refuses it, and a refused change takes no
        // number; an accepted one counts as delivered here once it is tagged.
        if (log.size >= room) return Offered.Answered(Verdict.Refused(Refusal.NO_ROOM_FOR_HISTORY))
        val own = ownSeq
        if (own == ULong.MAX_VALUE) return Offered.Answered(Verdict.Refused(Refusal.NUMBERS_EXHAUSTED))
        val seq = own + 1uL
        val dot = replica.entry(seq)
        val raised = causalRaise(delivered, listOf(dot))
        val message = build(tagOf(dot, raised)) ?: return Offered.Unbuilt
        delivered = raised
        log += CausalLogged(dot, message, released = false)
        return Offered.Answered(Verdict.Accepted(Dot(replica, seq)))
    }

    /**
     * The host committed the journal record of [dot]: every accepted change of
     * this incarnation up to it may now leave, in the order it was accepted.
     * These are the messages to send to the group.
     */
    fun committed(dot: Dot): List<M> {
        if (dot.replica != replica) return emptyList()
        val out = mutableListOf<M>()
        for (logged in log) {
            if (!logged.released && Replica.of(logged.dot) == replica && logged.dot.seq <= dot.seq) {
                logged.released = true
                out += logged.message
            }
        }
        return out
    }

    /**
     * A change that arrived, tagged [tag]: what it lets this copy deliver, in a
     * causal order. A change delivered already is dropped, the next change of
     * its author whose context is delivered is delivered with every held
     * change that then follows, and any other is held. An echo of this copy's
     * own change is dropped.
     */
    fun receive(tag: CausalTag, message: M): CausalReceived<M> {
        rejectionOf(tag)?.let { return CausalReceived.Rejected(it) }
        val sender = tag.vector[0]
        val author = Replica.of(sender)
        if (author == replica) return CausalReceived.Messages(emptyList())
        if (!holdsMember(author)) return CausalReceived.Rejected(Rejection.NotInView(author))
        noteKnown(author, tag.vector)
        // §mesh-10.12, the rule: a change at or below its author's column is
        // dropped, the next one whose context is delivered is delivered with
        // every held change that then follows, and any other is held.
        val admitted = when (val result = causalAdmit(delivered, sender, tag.vector)) {
            is AlgorithmResult.Failed -> return CausalReceived.Rejected(Rejection.Rule(result.error))
            is AlgorithmResult.Ok -> result.value
        }
        val out = mutableListOf<M>()
        when (admitted) {
            CausalAdmission.DROP -> {}
            CausalAdmission.HOLD -> {
                // A hold past the log's room is let go: the change is still
                // the sender's, and a later vector exchange brings it again.
                if (held.none { it.dot == sender } && held.size < room) {
                    held += CausalHeld(sender, tag.vector, message)
                }
            }
            CausalAdmission.DELIVER -> {
                deliver(sender, message, out)
                drainHeld(out)?.let { return CausalReceived.Rejected(Rejection.Rule(it)) }
                prune()
            }
        }
        return CausalReceived.Messages(out)
    }

    /**
     * This copy's `causalVector`: the epoch and its delivered vector, its own
     * entry first. Sent on a period and whenever a link comes back.
     */
    fun summary(): CausalTag = tagOf(replica.entry(ownSeq), delivered)

    /**
     * A member's `causalVector`: recorded as that member's latest known vector
     * (its mark, which moves the stable cut), and answered with every released
     * change of the log the vector lacks, in the order the log holds them (a
     * causal order), as they were sent.
     */
    fun receiveSummary(tag: CausalTag): CausalReceived<M> {
        rejectionOf(tag)?.let { return CausalReceived.Rejected(it) }
        val author = Replica.of(tag.vector[0])
        if (author == replica) return CausalReceived.Messages(emptyList())
        if (!holdsMember(author)) return CausalReceived.Rejected(Rejection.NotInView(author))
        // The mark: what the member says it delivered.
        noteKnown(author, tag.vector)
        val answer = log.filter { it.released && causalLacks(tag.vector, it.dot) }.map { it.message }
        prune()
        return CausalReceived.Messages(answer)
    }

    /**
     * The stable cut: the pointwise minimum, over the view's members, of the
     * latest delivered vector known from each, with a member not heard from at
     * 0 (`causal_cut`). This copy's own row comes first, then every other
     * member's in the order of its replica.
     */
    fun stableCut(): List<CausalEntryPayload> {
        val rows = listOf(replica to delivered) + known.keys.sorted().map { it to known.getValue(it) }
        val marks = rows.flatMap { (holder, vector) ->
            vector.map { e ->
                CausalKnownPayload(holderHi = holder.hi, holderLo = holder.lo, hi = e.hi, lo = e.lo, seq = e.seq)
            }
        }
        return causalCut(group.view.size.toUInt(), marks)
    }

    /** The tag a message of this copy carries: the epoch, and [own] first, then every other entry of [vector]. */
    private fun tagOf(own: CausalEntryPayload, vector: List<CausalEntryPayload>): CausalTag =
        CausalTag(group.epoch, listOf(own) + vector.filter { Replica.of(it) != replica })

    /** Why [tag] is not a tag of this view, or `null` when it is one, its sender entry first. */
    private fun rejectionOf(tag: CausalTag): Rejection? =
        when {
            tag.epoch != group.epoch -> Rejection.OtherEpoch(tag.epoch)
            tag.vector.isEmpty() || tag.vector.size > CAUSAL_VECTOR_BOUND -> Rejection.Malformed
            else -> null
        }

    /**
     * Whether [author] is held to the view: a member already heard from, or one
     * more while the view has a member no replica holds yet. Until view changes
     * land the view is epoch 0's, and each of its members is held by the first
     * replica that presents itself for it; a replica past the view's size is a
     * second incarnation of a member, which is not delivered.
     */
    private fun holdsMember(author: Replica): Boolean =
        author in known || 1 + known.size < group.view.size

    /** Raise the latest vector known of [author] by [vector]. */
    private fun noteKnown(author: Replica, vector: List<CausalEntryPayload>) {
        known[author] = causalRaise(known[author] ?: emptyList(), vector)
    }

    /** Deliver the change [dot]: raise the delivered vector by it, log it and hand its message on. */
    private fun deliver(dot: CausalEntryPayload, message: M, out: MutableList<M>) {
        delivered = causalRaise(delivered, listOf(dot))
        log += CausalLogged(dot, message, released = true)
        out += message
    }

    /**
     * Deliver every held change that is now deliverable, each after the ones
     * its context names, and let go of every held change now delivered. `null`
     * when every rule answered; the error of the one that did not, otherwise.
     */
    private fun drainHeld(out: MutableList<M>): AlgorithmError? {
        var delivering = true
        while (delivering) {
            delivering = false
            var index = 0
            while (!delivering && index < held.size) {
                val candidate = held[index]
                val admitted = when (val result = causalAdmit(delivered, candidate.dot, candidate.context)) {
                    is AlgorithmResult.Failed -> return result.error
                    is AlgorithmResult.Ok -> result.value
                }
                when (admitted) {
                    CausalAdmission.DROP -> held.removeAt(index)
                    CausalAdmission.HOLD -> index += 1
                    CausalAdmission.DELIVER -> {
                        held.removeAt(index)
                        deliver(candidate.dot, candidate.message, out)
                        delivering = true
                    }
                }
            }
        }
        return null
    }

    /**
     * Drop every logged change the stable cut covers: it is delivered at every
     * member of the view, so no repair can ask for it.
     */
    private fun prune() {
        val cut = stableCut()
        log.removeAll { !causalLacks(cut, it.dot) }
    }
}

/** How an envelope stands to the causal route's tag, keys 19 (`causal_epoch`) and 20 (`causal_vector`). */
sealed class EnvelopeTag {
    /** It carries neither key. */
    data object Untagged : EnvelopeTag()

    /** It carries both: [tag]. */
    data class Tagged(val tag: CausalTag) : EnvelopeTag()

    /** It carries one without the other, which no core writes. */
    data object HalfTagged : EnvelopeTag()
}

/** The tag [envelope] carries in keys 19 and 20. */
fun envelopeTag(envelope: Envelope): EnvelopeTag {
    val epoch = envelope.causal_epoch
    val vector = envelope.causal_vector
    return when {
        epoch == null && vector == null -> EnvelopeTag.Untagged
        epoch != null && vector != null -> EnvelopeTag.Tagged(CausalTag(epoch, vector))
        else -> EnvelopeTag.HalfTagged
    }
}

/**
 * [envelope] with [tag] in keys 19 and 20. A vector past its 64 entries is
 * refused when the envelope is written, as every entry past its bound is.
 */
fun stamp(envelope: Envelope, tag: CausalTag): Envelope =
    envelope.copy(causal_epoch = tag.epoch, causal_vector = tag.vector)

/** The number [vector] gives [replica], 0 when it names none. */
private fun seen(vector: List<CausalEntryPayload>, replica: Replica): ULong =
    vector.firstOrNull { Replica.of(it) == replica }?.seq ?: 0uL
