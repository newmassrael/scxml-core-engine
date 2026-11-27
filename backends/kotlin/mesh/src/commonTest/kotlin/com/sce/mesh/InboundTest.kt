// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The receive half on an ordered binding, case for case with the reply cases of
// backends/rust/mesh/src/inbound.rs.

package com.sce.mesh

import com.sce.generated.envelope.Envelope
import com.sce.generated.envelope.EnvelopeEventTypeText
import com.sce.generated.envelope.EnvelopeIdBytes
import com.sce.generated.envelope.EnvelopeSourceText
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class InboundTest {
    private val orderedOnly = Delivery(dedup = false, ordered = true)
    private val orderedAndDeduplicated = Delivery(dedup = true, ordered = true)

    /** An envelope of [pattern] from `a`, its id all [n], numbered [sequence]. */
    private fun wire(pattern: PatternKind, n: Int, sequence: ULong?): ByteArray =
        Envelope(
            id = EnvelopeIdBytes.of(ByteArray(16) { n.toByte() })!!,
            source = EnvelopeSourceText.of("a")!!,
            event_type = EnvelopeEventTypeText.of("evt")!!,
            pattern = pattern,
            datacontenttype = PayloadCodec.NONE,
            sequence_no = sequence,
        ).encodeToByteArray() ?: fail("the envelope did not encode")

    private fun outcome(admission: Admission): Outcome = when (admission) {
        is Admission.Admitted -> admission.outcome
        is Admission.Refused -> fail("refused: ${admission.error}")
    }

    private fun numbers(admission: Admission): List<ULong?> = outcome(admission).released.map { it.sequence_no }

    /**
     * SCE_MESH.md section 10.6.4: a reply and a field notification are not in their sender's sequence.
     * Unnumbered on an ordered binding, each is released at once, past a gap the stream is waiting on,
     * and the duplicate window still drops a second copy. Red if the receiver asks them for a number:
     * each is then refused as `Unstamped`.
     */
    @Test
    fun anUnnumberedReplyOrFieldNotificationPassesAnOrderedBinding() {
        val inbound = Inbound(8u, 50)
        val both = orderedAndDeduplicated
        assertEquals(listOf<ULong?>(1uL), numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 1, 1uL), both, 0)))
        assertTrue(outcome(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 3, 3uL), both, 0)).released.isEmpty())
        for ((n, pattern) in listOf(10 to PatternKind.RPC_REPLY, 11 to PatternKind.FIELD_NOTIFY)) {
            val bytes = wire(pattern, n, null)
            assertEquals(
                listOf(pattern),
                outcome(inbound.admit("a", bytes, both, 0)).released.map { it.pattern },
                "$pattern is released at once",
            )
            assertTrue(
                outcome(inbound.admit("a", bytes, both, 0)).released.isEmpty(),
                "a second copy of $pattern is a duplicate",
            )
        }
        // The stream still waits for 2, and 3 follows it.
        assertEquals(
            listOf<ULong?>(2uL, 3uL),
            numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 2, 2uL), both, 0)),
        )
    }

    /**
     * SCE_MESH.md section 10.6.4: a reply its sender numbered anyway is passed as one that carries no
     * number. It neither anchors the stream nor is held behind a gap, and its number is never waited
     * for, so no gap times out. Red if a numbered reply is ordered: the first one would be held ahead
     * of the stream, and the gap below it would time out as lost.
     */
    @Test
    fun aNumberedReplyTakesNoPlaceInTheStream() {
        val inbound = Inbound(8u, 50)
        assertEquals(
            listOf(PatternKind.RPC_REPLY),
            outcome(inbound.admit("a", wire(PatternKind.RPC_REPLY, 9, 9uL), orderedOnly, 0)).released.map { it.pattern },
        )
        for (n in 1..2) {
            assertEquals(
                listOf<ULong?>(n.toULong()),
                numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, n, n.toULong()), orderedOnly, 10)),
            )
        }
        assertEquals(
            1,
            outcome(inbound.admit("a", wire(PatternKind.RPC_REPLY, 20, 3uL), orderedOnly, 20)).released.size,
        )
        assertEquals(
            listOf<ULong?>(3uL),
            numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 3, 3uL), orderedOnly, 30)),
        )
        val later = outcome(inbound.tick(10_000))
        assertTrue(later.released.isEmpty() && later.signals.isEmpty(), "no gap times out: ${later.signals}")
    }

    /**
     * SCE_MESH.md section 10.6.4: a stream the receiver has no state for is expected from its first
     * number, whatever arrives first, so an envelope that overtook the first waits for it and both
     * are released in order. Red if the first envelope that arrives anchors the stream: 2 is then
     * released at once and 1 is dropped as a straggler, told to no one.
     */
    @Test
    fun aFreshStreamWaitsForItsFirstNumber() {
        val inbound = Inbound(8u, 50)
        val overtaking = outcome(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 2, 2uL), orderedOnly, 0))
        assertTrue(overtaking.released.isEmpty(), "2 waits for 1")
        assertEquals(
            listOf<ULong?>(1uL, 2uL),
            numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 1, 1uL), orderedOnly, 10)),
        )
        val later = outcome(inbound.tick(10_000))
        assertTrue(later.released.isEmpty() && later.signals.isEmpty(), "no gap is left: ${later.signals}")
    }

    /**
     * SCE_MESH.md section 10.6.4: a stream joined after its start (a receiver whose state was lost)
     * waits for the numbers below its first arrival as for any gap, and once the gap has waited out its
     * timeout tells them lost and releases what it held. Red if the first arrival anchors the stream:
     * 7 is then released at once and 1 to 6 are never told.
     */
    @Test
    fun aStreamJoinedAfterItsStartTellsTheNumbersBeforeItLost() {
        val inbound = Inbound(8u, 50)
        val first = outcome(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 7, 7uL), orderedOnly, 0))
        assertTrue(first.released.isEmpty(), "7 waits for the numbers below it")
        val early = outcome(inbound.tick(49))
        assertTrue(early.released.isEmpty() && early.signals.isEmpty(), "the gap has not waited its timeout")
        val ended = outcome(inbound.tick(50))
        assertEquals(listOf<Signal>(Signal.OrderingGap("a", 1uL, 6uL)), ended.signals.toList())
        assertEquals(listOf<ULong?>(7uL), ended.released.map { it.sequence_no })
        // 6 was told lost, so a copy that comes late is a straggler.
        val straggler = outcome(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 6, 6uL), orderedOnly, 60))
        assertTrue(straggler.released.isEmpty() && straggler.signals.isEmpty())
        assertEquals(
            listOf<ULong?>(8uL),
            numbers(inbound.admit("a", wire(PatternKind.FIRE_FORGET, 8, 8uL), orderedOnly, 60)),
        )
    }

    /** Every other pattern keeps its place: an unnumbered one on an ordered binding is refused. */
    @Test
    fun anUnnumberedSendOnAnOrderedBindingIsStillRefused() {
        assertEquals(
            Admission.Refused(AdmitError.Unstamped),
            Inbound(8u, 50).admit("a", wire(PatternKind.FIRE_FORGET, 1, null), orderedOnly, 0),
        )
    }
}
