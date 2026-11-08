// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.12
//
// The tests of docs/adr/0085, section 9.2 for the Kotlin core's causal layer,
// case for case with the tests of backends/rust/mesh/src/causal.rs. Each names
// the change to the rule that turns it red.

package com.sce.mesh

import com.sce.forge.runtime.SceCursor
import com.sce.generated.causal_entry.CausalEntryPayload
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class CausalLayerTest {
    private val view = CausalGroup("people", 0uL, listOf("phone/list", "laptop/list", "tablet/list"))
    private val pair = CausalGroup("pair", 0uL, listOf("phone/list", "laptop/list"))

    /** A message as the tests carry it: the tag it was sent with and a name. */
    private data class Msg(val tag: CausalTag, val name: Int)

    private fun replica(n: Int): Replica = Replica.fromBytes(ByteArray(16) { n.toByte() })

    private fun layer(group: CausalGroup, n: Int, room: Int): CausalLayer<Msg> =
        CausalLayer(group, CausalStart.NewIncarnation(replica(n)), room)

    /** The verdict an offer of the change [name] is answered with. */
    private fun verdictOf(at: CausalLayer<Msg>, name: Int): Verdict =
        when (val offered = at.offer { tag -> Msg(tag, name) }) {
            is Offered.Answered -> offered.verdict
            Offered.Unbuilt -> fail("a message is always built here")
        }

    /** Offer the change [name] and commit its journal record: the message that leaves for the group. */
    private fun make(at: CausalLayer<Msg>, name: Int): Msg {
        val verdict = verdictOf(at, name)
        val dot = (verdict as? Verdict.Accepted)?.dot ?: fail("expected the change accepted, got $verdict")
        val sent = at.committed(dot)
        assertEquals(1, sent.size, "one change released per commit")
        return sent[0]
    }

    private fun taken(received: CausalReceived<Msg>): List<Msg> =
        when (received) {
            is CausalReceived.Messages -> received.messages
            is CausalReceived.Rejected -> fail("expected it taken in, got ${received.rejection}")
        }

    /** Give [message] to [at]: the names it delivers, in order. */
    private fun give(at: CausalLayer<Msg>, message: Msg): List<Int> = taken(at.receive(message.tag, message)).map { it.name }

    /** Give [to] the `causalVector` of [from]: what [to] answers with. */
    private fun exchange(from: CausalLayer<Msg>, to: CausalLayer<Msg>): List<Msg> = taken(to.receiveSummary(from.summary()))

    private fun column(at: CausalLayer<Msg>, of: Replica): ULong =
        at.delivered.firstOrNull { Replica.of(it) == of }?.seq ?: 0uL

    /** Drop. Red if `dot.seq <= seen` becomes `<` in causal_admit: the change given again is then held, not dropped. */
    @Test
    fun aChangeGivenAgainIsDroppedAndNotHeld() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val c1 = make(p, 1)
        assertEquals(listOf(1), give(q, c1))
        assertEquals(emptyList(), give(q, c1))
        assertEquals(0, q.heldLen)
        assertEquals(1uL, column(q, replica(1)))
    }

    /** Deliver. Red if the delivered vector is not raised by the change delivered: the author's next change is then held. */
    @Test
    fun theNextChangeIsDeliveredOnceAndRaisesItsAuthorsColumn() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        assertEquals(listOf(1), give(q, c1))
        assertEquals(1uL, column(q, replica(1)))
        assertEquals(listOf(2), give(q, c2))
        assertEquals(2uL, column(q, replica(1)))
    }

    /**
     * Hold, a gap of the author. Red if `dot.seq === seen + 1` is weakened to
     * `dot.seq > seen` in causal_admit: the change past the gap is then
     * delivered first.
     */
    @Test
    fun aChangePastAGapOfItsAuthorIsHeldUntilTheGapFills() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        assertEquals(emptyList(), give(q, c2))
        assertEquals(1, q.heldLen)
        assertEquals(listOf(1, 2), give(q, c1))
        assertEquals(0, q.heldLen)
    }

    /**
     * Hold, an uncovered context. Red if the check `C[b] <= D[b]` is removed
     * from causal_admit: the change is then delivered before the change of
     * another author it depends on.
     */
    @Test
    fun aChangeWhoseContextIsNotDeliveredIsHeldUntilItIs() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val r = layer(view, 3, 16)
        val c1 = make(p, 1)
        assertEquals(listOf(1), give(r, c1))
        val d1 = make(r, 10)
        assertEquals(emptyList(), give(q, d1))
        assertEquals(listOf(1, 10), give(q, c1))
    }

    /**
     * No gap timeout. Red if anything other than the change before it releases
     * a held change, as a gap timeout that fast-forwards the author's column
     * would.
     */
    @Test
    fun aHeldChangeStaysHeldHoweverManyVectorsPass() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val r = layer(view, 3, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        assertEquals(emptyList(), give(q, c2))
        repeat(1000) {
            exchange(r, q)
            exchange(q, r)
        }
        assertEquals(1, q.heldLen)
        assertEquals(0uL, column(q, replica(1)))
        assertEquals(listOf(1, 2), give(q, c1))
    }

    /** Repair by vector. Red if what a vector lacks is computed as empty in causal_lacks: the lost change is then never sent again. */
    @Test
    fun aLostChangeIsReceivedAfterOneVectorExchange() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        make(p, 1)
        val answer = exchange(q, p)
        assertEquals(1, answer.size)
        assertEquals(listOf(1), give(q, answer[0]))
    }

    /**
     * The stable cut. Red if the minimum becomes a maximum in causal_cut (the
     * cut is then {p:2}), or if a member whose vector is unknown is left out
     * (the cut is then not empty before the third member is heard from).
     */
    @Test
    fun theStableCutIsTheMinimumOverTheViewWithAnUnknownMemberAtZero() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val r = layer(view, 3, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        give(q, c1)
        give(q, c2)
        exchange(q, p)
        assertEquals(emptyList<CausalEntryPayload>(), p.stableCut())
        give(r, c1)
        exchange(r, p)
        assertEquals(listOf(replica(1).entry(1uL)), p.stableCut())
    }

    /**
     * The mark. Red if a received vector is not recorded as its member's latest
     * (the `noteKnown` of receiveSummary removed): members that never wrote
     * then never move the cut, and the log keeps the change.
     */
    @Test
    fun membersThatNeverWroteMoveTheCutByTheirVectorsAlone() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val r = layer(view, 3, 16)
        val c1 = make(p, 1)
        give(q, c1)
        give(r, c1)
        assertEquals(1, p.logLen)
        exchange(q, p)
        exchange(r, p)
        assertEquals(0, p.logLen)
    }

    /**
     * Retention. Red if the log is pruned at the copy's own delivered vector
     * instead of the cut: the change a member still lacks is then gone, and
     * its repair is not answered.
     */
    @Test
    fun theLogDropsAChangeAtTheCutAndAnswersARepairBeforeIt() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val r = layer(view, 3, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        give(q, c1)
        give(q, c2)
        give(r, c1)
        exchange(q, p)
        val answer = exchange(r, p)
        assertEquals(1, p.logLen, "c1 is stable, c2 is not")
        assertEquals(1, answer.size)
        assertEquals(listOf(2), give(r, answer[0]))
    }

    /**
     * The verdict. Red if the number is taken before the refusal: the next
     * accepted change then leaves a gap, and the peer holds it.
     */
    @Test
    fun anOfferPastTheRoomIsRefusedAndTakesNoNumber() {
        val p = layer(pair, 1, 1)
        val q = layer(pair, 2, 16)
        val c1 = make(p, 1)
        assertEquals(Verdict.Refused(Refusal.NO_ROOM_FOR_HISTORY), verdictOf(p, 99))
        assertEquals(1uL, p.ownSeq)
        assertEquals(listOf(1), give(q, c1))
        exchange(q, p)
        assertEquals(0, p.logLen, "stability frees room")
        val c2 = make(p, 2)
        assertEquals(replica(1).entry(2uL), c2.tag.vector[0])
        assertEquals(listOf(2), give(q, c2))
    }

    /** A message that cannot be built takes no number, and the next one is the number it would have had. */
    @Test
    fun anOfferWhoseMessageCannotBeBuiltTakesNoNumber() {
        val p = layer(view, 1, 16)
        assertEquals(Offered.Unbuilt, p.offer { null })
        assertEquals(0uL, p.ownSeq)
        assertEquals(0, p.logLen)
        assertEquals(replica(1).entry(1uL), make(p, 1).tag.vector[0])
    }

    /**
     * Write-ahead. Red if the layer releases a change when it tags it: a repair
     * asked before the journal commit is then answered with it.
     */
    @Test
    fun anAcceptedChangeLeavesOnlyOnceItsJournalRecordIsCommitted() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val verdict = verdictOf(p, 1)
        val dot = (verdict as? Verdict.Accepted)?.dot ?: fail("expected the change accepted, got $verdict")
        assertEquals(1uL, dot.seq)
        assertTrue(exchange(q, p).isEmpty())
        assertEquals(1, p.committed(dot).size)
        assertTrue(p.committed(dot).isEmpty(), "a change is released once")
        assertEquals(1, exchange(q, p).size)
        // A dot of another replica releases nothing of this one.
        assertTrue(p.committed(Dot(replica(9), 1uL)).isEmpty())
    }

    /**
     * Incarnation. Red if a copy without its journal reuses its previous
     * identity, or continues from a number any peer reports: its next change
     * then carries the dot of the change an offline member holds.
     */
    @Test
    fun aCopyWithoutItsJournalBeginsANewIncarnation() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val c1 = make(p, 1)
        val c2 = make(p, 2)
        give(q, c1)
        // c2 reached only the tablet, which is offline now. The phone is
        // restored from a save taken after c1: it cannot know its highest.
        // A copy whose journal is its own continues from it.
        val record = CausalJournalRecord(replica(1), listOf(replica(1).entry(2uL)))
        assertEquals(CausalStart.Journal(record), CausalStart.fromJournal(record, replica(9)))
        val resumed = CausalLayer<Msg>(view, CausalStart.Journal(record), 16)
        assertEquals(replica(1).entry(3uL), make(resumed, 4).tag.vector[0])
        val restored = CausalLayer<Msg>(view, CausalStart.fromJournal(null, replica(9)), 16)
        assertEquals(replica(9), restored.replica)
        val next = make(restored, 3)
        val dots = listOf(c1, c2, next).map { Dot.of(it.tag.vector[0]) }.toSet()
        assertEquals(3, dots.size, "no dot names two contents")
        assertEquals(replica(9).entry(1uL), next.tag.vector[0])
        // Until view changes land, the laptop, which knows the phone's first
        // incarnation and the tablet, does not deliver the second.
        val r = layer(view, 3, 16)
        exchange(r, q)
        assertEquals(CausalReceived.Rejected(Rejection.NotInView(replica(9))), q.receive(next.tag, next))
    }

    /** The view. Red if the epoch check is removed: a change of another epoch is then delivered. */
    @Test
    fun aChangeOfAnotherEpochIsRejectedAndNotDelivered() {
        val p = layer(view, 1, 16)
        val q = layer(view, 2, 16)
        val sent = make(p, 1)
        val tag = sent.tag.copy(epoch = 1uL)
        assertEquals(CausalReceived.Rejected(Rejection.OtherEpoch(1uL)), q.receive(tag, Msg(tag, sent.name)))
        assertEquals(
            CausalReceived.Rejected(Rejection.OtherEpoch(1uL)),
            q.receiveSummary(CausalTag(1uL, listOf(replica(1).entry(0uL)))),
        )
        assertEquals(0uL, column(q, replica(1)))
    }

    /** A vector with no sender entry, or past the bound of a view, is malformed and not taken in. */
    @Test
    fun aVectorWithNoSenderOrPastItsBoundIsMalformed() {
        val q = layer(view, 2, 16)
        assertEquals(CausalReceived.Rejected(Rejection.Malformed), q.receiveSummary(CausalTag(0uL, emptyList())))
        val long = (1..CAUSAL_VECTOR_BOUND + 1).map { replica(it).entry(1uL) }
        assertEquals(CausalReceived.Rejected(Rejection.Malformed), q.receiveSummary(CausalTag(0uL, long)))
    }

    /** A replica's identity reads back from its bytes, its first eight the high half, big-endian. */
    @Test
    fun aReplicaIsItsSixteenBytesInTwoBigEndianHalves() {
        val bytes = ByteArray(16) { it.toByte() }
        val id = Replica.fromBytes(bytes)
        assertEquals(0x0001020304050607uL, id.hi)
        assertEquals(0x08090A0B0C0D0E0FuL, id.lo)
        assertTrue(bytes.contentEquals(id.toBytes()))
        assertTrue(Replica(1uL, 0uL) < Replica(ULong.MAX_VALUE, 0uL), "the halves compare unsigned")
    }

    private fun plainEnvelope(): Envelope = Envelope(
        id = ByteArray(16) { 7 },
        source = "list",
        event_type = "",
        pattern = PatternKind.CAUSAL_VECTOR,
        datacontenttype = PayloadCodec.NONE,
    )

    /**
     * The wire. A tag is written as keys 19 and 20, the highest keys, so it
     * ends the map: the epoch, then an array of one map per entry, its keys as
     * text in RFC 8949 order (`hi`, `lo`, `seq`). The same tail the Rust core's
     * test asserts. Red if a core numbers either key otherwise, or writes an
     * entry's fields in another order.
     */
    @Test
    fun aTagIsWrittenAsKeys19And20AndReadBack() {
        val tag = CausalTag(0uL, listOf(CausalEntryPayload(hi = 1uL, lo = 2uL, seq = 3uL)))
        val bytes = stamp(plainEnvelope(), tag).encodeToByteArray() ?: fail("a vector within its bound encodes")
        val tail = listOf(
            0x13, 0x00, // key 19, epoch 0
            0x14, 0x81, // key 20, an array of one
            0xA3, // a map of three
            0x62, 0x68, 0x69, 0x01, // "hi": 1
            0x62, 0x6C, 0x6F, 0x02, // "lo": 2
            0x63, 0x73, 0x65, 0x71, 0x03, // "seq": 3
        ).map { it.toByte() }.toByteArray()
        val hex = bytes.joinToString("") { (it.toInt() and 0xFF).toString(16).padStart(2, '0') }
        assertTrue(bytes.size >= tail.size && bytes.copyOfRange(bytes.size - tail.size, bytes.size).contentEquals(tail), hex)
        val decoded = Envelope.decode(SceCursor(bytes)) ?: fail("the envelope reads back")
        assertEquals(EnvelopeTag.Tagged(tag), envelopeTag(decoded))
        assertEquals(EnvelopeTag.Untagged, envelopeTag(plainEnvelope()))
        assertEquals(EnvelopeTag.HalfTagged, envelopeTag(plainEnvelope().copy(causal_epoch = 0uL)))
    }

    /** A small deterministic generator for the world below. */
    private class Rng(private var state: ULong) {
        fun step(): ULong {
            state = state xor (state shl 13)
            state = state xor (state shr 7)
            state = state xor (state shl 17)
            return state
        }

        fun below(n: Int): Int = (step() % n.toULong()).toInt()

        fun chance(percent: Int): Boolean = step() % 100uL < percent.toULong()
    }

    private sealed class Packet {
        data class Change(val message: Msg) : Packet()

        data class Vector(val from: Int, val tag: CausalTag) : Packet()
    }

    /**
     * The world. Three copies on links that lose, duplicate and reorder deliver
     * every change exactly once, in an order that is a linear extension of the
     * contexts, with no gap skipped, and their logs drop to the cut once the
     * links are quiet. Red if the hold branch delivers (the order check fails),
     * or if repair is removed (a lost change then never arrives).
     */
    @Test
    fun threeCopiesOnLossyReorderingLinksDeliverEveryChangeOnceInCausalOrder() {
        val changesEach = 20
        for (seed in listOf(0x5EED0001uL, 0x5EED0002uL, 0x5EED0003uL)) {
            val rng = Rng(seed)
            val copies = (1..3).map { layer(view, it, 1000) }
            val delivered = List(3) { mutableListOf<Msg>() }
            val made = IntArray(3)
            val wire = mutableListOf<Pair<Int, Packet>>()

            fun send(target: Int, packet: Packet, lossy: Boolean) {
                if (lossy && rng.chance(30)) return
                if (packet is Packet.Change && lossy && rng.chance(10)) wire += Pair(target, packet)
                wire += Pair(target, packet)
            }

            fun deliverOne(lossy: Boolean) {
                val (target, packet) = wire.removeAt(rng.below(wire.size))
                when (packet) {
                    is Packet.Change ->
                        delivered[target] += taken(copies[target].receive(packet.message.tag, packet.message))
                    is Packet.Vector -> {
                        for (message in taken(copies[target].receiveSummary(packet.tag))) {
                            send(packet.from, Packet.Change(message), lossy)
                        }
                    }
                }
            }

            // The busy part: changes made, delivered out of order, lost and
            // repeated, with vectors exchanged now and then.
            while (made.any { it < changesEach } || wire.isNotEmpty()) {
                when (rng.below(10)) {
                    0, 1, 2 -> {
                        val at = rng.below(3)
                        if (made[at] < changesEach) {
                            made[at] += 1
                            val message = make(copies[at], at * 1000 + made[at])
                            delivered[at] += message
                            for (to in 0 until 3) {
                                if (to != at) send(to, Packet.Change(message), true)
                            }
                        }
                    }
                    3 -> {
                        val from = rng.below(3)
                        val tag = copies[from].summary()
                        for (to in 0 until 3) {
                            if (to != from) send(to, Packet.Vector(from, tag), true)
                        }
                    }
                    else -> if (wire.isNotEmpty()) deliverOne(true)
                }
            }

            // The quiet part: links that lose nothing, a vector from each copy
            // per round, until every copy holds every change.
            repeat(20) {
                val tags = copies.map { it.summary() }
                for ((from, tag) in tags.withIndex()) {
                    for (to in 0 until 3) {
                        if (to != from) send(to, Packet.Vector(from, tag), false)
                    }
                }
                while (wire.isNotEmpty()) deliverOne(false)
            }

            val label = "seed ${seed.toString(16)}"
            for ((at, order) in delivered.withIndex()) {
                assertEquals(3 * changesEach, order.size, "$label: copy $at delivers every change")
                val columns = mutableMapOf<Replica, ULong>()
                for (message in order) {
                    val dot = message.tag.vector[0]
                    val author = Replica.of(dot)
                    assertEquals(
                        (columns[author] ?: 0uL) + 1uL,
                        dot.seq,
                        "$label: copy $at delivers ${message.name} once and skips no gap",
                    )
                    for (needed in message.tag.vector.drop(1)) {
                        assertTrue(
                            needed.seq <= (columns[Replica.of(needed)] ?: 0uL),
                            "$label: copy $at delivers ${message.name} before its context",
                        )
                    }
                    columns[author] = dot.seq
                }
                assertEquals(0, copies[at].heldLen, "$label: nothing held")
                assertEquals(0, copies[at].logLen, "$label: the log is at the cut")
            }
        }
    }
}
