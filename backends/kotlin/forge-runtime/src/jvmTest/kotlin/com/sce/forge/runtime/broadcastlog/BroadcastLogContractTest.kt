// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin arm of the sequential contract of the bounded broadcast log (RFC
// §synth-5-S; ADR 0062 section 2): the unit tests of the Rust row
// (backends/rust/forge-runtime/src/broadcast_log.rs) carried over, then the
// refusals and the edges this arm adds. Everything here runs on one thread, so
// each test states what the log answers and not what a schedule gave.

package com.sce.forge.runtime.broadcastlog

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Three slots retain two entries; an entry is two words. */
private fun smallLog() = BroadcastLog(words = 2, slots = 3)

private fun entryOf(n: Long) = longArrayOf(n, n)

/** What a read answered and what it left in the buffer. */
private class Answer(val read: Read, val out: LongArray)

/** A read into a buffer that holds a value no entry has, so an untouched buffer shows. */
private fun BroadcastLog.readOnce(reader: Reader): Answer {
    val out = LongArray(words) { -1L }
    return Answer(read(reader, out), out)
}

private fun BroadcastLog.assertItem(reader: Reader, seq: Long, expected: LongArray) {
    val answer = readOnce(reader)
    assertEquals(Read.Item(seq), answer.read)
    assertContentEquals(expected, answer.out)
}

class BroadcastLogContractTest {
    @Test
    fun anEmptyLogIsEmptyToEveryReader() {
        val log = smallLog()
        val reader = log.readerOldest()
        assertEquals(Read.Empty, log.readOnce(reader).read)
        assertEquals(0L, reader.cursor)
        assertEquals(0L, log.head())
    }

    @Test
    fun entriesComeBackInOrderWithTheirWords() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        assertEquals(0L, publisher.publish(entryOf(10)))
        assertEquals(1L, publisher.publish(entryOf(11)))
        log.assertItem(reader, 0, entryOf(10))
        log.assertItem(reader, 1, entryOf(11))
        assertEquals(Read.Empty, log.readOnce(reader).read)
        assertEquals(0L, reader.lost)
    }

    @Test
    fun theWindowIsExactlyTheRetention() {
        // 3 slots retain 2 entries.
        val log = smallLog()
        assertEquals(2L, log.retention)
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 3L) {
            publisher.publish(entryOf(n))
        }
        // h = 3: the window is [1, 3), so a reader at 0 is one behind it.
        val behind = log.readerAt(0)
        assertEquals(Read.Lagged(1), log.readOnce(behind).read)
        assertEquals(1L, behind.cursor)
        assertEquals(1L, behind.lost)
        log.assertItem(behind, 1, entryOf(1))
        // A reader at the oldest retained entry lost nothing.
        val oldest = log.readerOldest()
        assertEquals(1L, oldest.cursor)
        log.assertItem(oldest, 1, entryOf(1))
        assertEquals(0L, oldest.lost)
    }

    @Test
    fun readersDoNotShareACursorOrALoss() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 5L) {
            publisher.publish(entryOf(n))
        }
        val a = log.readerAt(0)
        val b = log.readerAt(3)
        assertEquals(Read.Lagged(3), log.readOnce(a).read)
        log.assertItem(b, 3, entryOf(3))
        assertEquals(3L, a.lost)
        assertEquals(0L, b.lost)
        assertEquals(4L, b.cursor)
    }

    @Test
    fun aReaderAtTheHeadSeesOnlyWhatComesAfter() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        publisher.publish(entryOf(0))
        val reader = log.readerHead()
        assertEquals(Read.Empty, log.readOnce(reader).read)
        publisher.publish(entryOf(1))
        log.assertItem(reader, 1, entryOf(1))
        // A position past the head is clamped to it.
        assertEquals(2L, log.readerAt(99).cursor)
    }

    @Test
    fun aDeferredReaderStartsAtTheOldestOfItsFirstReadAndIsToldOfNothing() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        // Both readers open on the empty log, one at the oldest of that moment and
        // one deferred; then five entries (0 to 4) are published. Three slots retain
        // two, so the oldest retained is entry 3.
        val eager = log.readerOldest()
        val deferred = log.readerDeferred()
        assertTrue(eager.isBound)
        assertFalse(deferred.isBound)
        for (n in 0L until 5L) {
            publisher.publish(entryOf(n))
        }
        // The eager reader holds 0, which is outside the window [3, 5): it is told it
        // missed 3.
        assertEquals(Read.Lagged(3), log.readOnce(eager).read)
        // The deferred reader binds at this read, at the oldest retained entry, so its
        // first answer is that entry and it has lost nothing.
        log.assertItem(deferred, 3, entryOf(3))
        assertTrue(deferred.isBound)
        assertEquals(0L, deferred.lost)
        assertEquals(4L, deferred.cursor)
        log.assertItem(deferred, 4, entryOf(4))
        assertEquals(Read.Empty, log.readOnce(deferred).read)
    }

    @Test
    fun aDeferredReaderOnALogWithNothingPublishedIsEmptyAndThenReadsFromTheStart() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerDeferred()
        assertEquals(Read.Empty, log.readOnce(reader).read)
        assertTrue(reader.isBound)
        assertEquals(0L, reader.cursor)
        publisher.publish(entryOf(0))
        log.assertItem(reader, 0, entryOf(0))
        assertEquals(0L, reader.lost)
    }

    @Test
    fun anUnboundReaderReportsCursorZeroAndNotBoundUntilItsFirstRead() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 3L) {
            publisher.publish(entryOf(n))
        }
        // Opening loads nothing: the head is 3 and the oldest retained is 1, and the
        // reader holds neither.
        val deferred = log.readerDeferred()
        assertFalse(deferred.isBound)
        assertEquals(0L, deferred.cursor)
        assertEquals(0L, deferred.lost)
        // The other three ways to open are bound at once.
        assertTrue(log.readerOldest().isBound)
        assertTrue(log.readerHead().isBound)
        assertTrue(log.readerAt(0).isBound)
        // The first read binds to the oldest retained entry, 1, and reads it.
        log.assertItem(deferred, 1, entryOf(1))
        assertTrue(deferred.isBound)
        assertEquals(2L, deferred.cursor)
    }

    @Test
    fun aDeferredReaderThatFallsBehindLaterIsToldAsAnyReaderIs() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 10L) {
            publisher.publish(entryOf(n))
        }
        val reader = log.readerDeferred()
        // h = 10: the window is [8, 10), and the bind took 8 with no loss.
        log.assertItem(reader, 8, entryOf(8))
        assertEquals(0L, reader.lost)
        for (n in 10L until 13L) {
            publisher.publish(entryOf(n))
        }
        // h = 13: the window is [11, 13) and the reader holds 9, so it missed 2.
        assertEquals(Read.Lagged(2), log.readOnce(reader).read)
        assertEquals(2L, reader.lost)
        assertEquals(11L, reader.cursor)
        log.assertItem(reader, 11, entryOf(11))
    }

    @Test
    fun aDeferredReaderIsTheRingsLateReaderWhichIsToldOfNothingHoweverMuchWasWritten() {
        // A ring of four retained entries: three are published, a reader is opened
        // deferred, ten more are published, and 13 have been published in all.
        val log = BroadcastLog(words = 2, slots = 5)
        assertEquals(4L, log.retention)
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 3L) {
            publisher.publish(entryOf(n))
        }
        val late = log.readerDeferred()
        for (n in 3L until 13L) {
            publisher.publish(entryOf(n))
        }
        // The oldest retained is 9, and the reader is told of nothing.
        val seen = ArrayList<Long>()
        while (true) {
            val answer = log.readOnce(late).read
            if (answer !is Read.Item) {
                assertEquals(Read.Empty, answer)
                break
            }
            seen.add(answer.seq)
        }
        assertEquals(listOf(9L, 10L, 11L, 12L), seen)
        assertEquals(0L, late.lost)
    }

    @Test
    fun thereIsOnePublisherAndItsSuccessorContinuesTheSequence() {
        val log = smallLog()
        val first = assertNotNull(log.publisher())
        assertNull(log.publisher(), "a second publisher is refused")
        first.publish(entryOf(0))
        first.release()
        val second = assertNotNull(log.publisher(), "the place was freed")
        assertEquals(1L, second.publish(entryOf(1)))
    }

    @Test
    fun publishingPastTheWindowKeepsTheNewestEntries() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 100L) {
            publisher.publish(entryOf(n))
        }
        val reader = log.readerOldest()
        log.assertItem(reader, 98, entryOf(98))
        log.assertItem(reader, 99, entryOf(99))
        assertEquals(Read.Empty, log.readOnce(reader).read)
    }

    @Test
    fun anEmptyReadLeavesTheCursorWhereItWas() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        publisher.publish(entryOf(0))
        val reader = log.readerHead()
        repeat(3) {
            assertEquals(Read.Empty, log.readOnce(reader).read)
            assertEquals(1L, reader.cursor)
        }
    }

    @Test
    fun theLossOfASlowReaderIsTheSumOfWhatItWasToldItMissed() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        for (n in 0L until 5L) {
            publisher.publish(entryOf(n))
        }
        val reader = log.readerAt(0)
        // h = 5: the window is [3, 5).
        assertEquals(Read.Lagged(3), log.readOnce(reader).read)
        log.assertItem(reader, 3, entryOf(3))
        for (n in 5L until 8L) {
            publisher.publish(entryOf(n))
        }
        // h = 8: the window is [6, 8) and the reader holds 4.
        assertEquals(Read.Lagged(2), log.readOnce(reader).read)
        assertEquals(5L, reader.lost)
        assertEquals(6L, reader.cursor)
    }

    @Test
    fun aLogOfOneRetainedEntryKeepsTheLatestAndNothingOlder() {
        val log = BroadcastLog(words = 1, slots = 2)
        assertEquals(1L, log.retention)
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        publisher.publish(longArrayOf(7))
        publisher.publish(longArrayOf(8))
        assertEquals(Read.Lagged(1), log.readOnce(reader).read)
        log.assertItem(reader, 1, longArrayOf(8))
        assertEquals(Read.Empty, log.readOnce(reader).read)
    }

    @Test
    fun anEntryOfManyWordsAndOneOfNoneRoundTrip() {
        val wide = BroadcastLog(words = 5, slots = 3)
        val publisher = assertNotNull(wide.publisher())
        val reader = wide.readerOldest()
        val entry = longArrayOf(Long.MIN_VALUE, -1, 0, 1, Long.MAX_VALUE)
        publisher.publish(entry)
        wide.assertItem(reader, 0, entry)

        val bare = BroadcastLog(words = 0, slots = 3)
        val barePublisher = assertNotNull(bare.publisher())
        val bareReader = bare.readerOldest()
        assertEquals(0L, barePublisher.publish(LongArray(0)))
        bare.assertItem(bareReader, 0, LongArray(0))
    }

    @Test
    fun aReleasedPublisherIsNotUsable() {
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        publisher.release()
        publisher.release()
        assertFailsWith<IllegalStateException> { publisher.publish(entryOf(0)) }
        assertNotNull(log.publisher(), "the place is free, once, after a double release")
    }

    @Test
    fun aLogRefusesAShapeItCannotRunAndABufferOfTheWrongSize() {
        assertFailsWith<IllegalArgumentException> { BroadcastLog(words = 2, slots = 1) }
        assertFailsWith<IllegalArgumentException> { BroadcastLog(words = 2, slots = 0) }
        assertFailsWith<IllegalArgumentException> { BroadcastLog(words = -1, slots = 3) }
        val log = smallLog()
        val publisher = assertNotNull(log.publisher())
        assertFailsWith<IllegalArgumentException> { publisher.publish(LongArray(1)) }
        assertFailsWith<IllegalArgumentException> { log.read(log.readerOldest(), LongArray(3)) }
        assertFailsWith<IllegalArgumentException> { log.readerAt(-1) }
    }

    @Test
    fun theSequenceStopsAtTheHorizonAndNeverGoesNegative() {
        val log = smallLog()
        log.startAt(HORIZON - 1)
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerHead()
        // The last entry that fits: its stamp, 2n + 2, is still positive.
        assertEquals(HORIZON - 1, publisher.publish(entryOf(5)))
        assertEquals(HORIZON, log.head())
        log.assertItem(reader, HORIZON - 1, entryOf(5))
        assertFailsWith<IllegalStateException> { publisher.publish(entryOf(6)) }
        assertEquals(HORIZON, log.head(), "the refused publish left the log as it was")
        assertEquals(Read.Empty, log.readOnce(reader).read)
    }

    @Test
    fun theHorizonIsInsideTheRangeTheStampsFitIn() {
        // done(n) = 2n + 2 is the larger stamp of an entry. The last entry
        // published is HORIZON - 1, and the stamp of HORIZON itself still fits.
        // Not a constant expression, so the last line is the wrap itself and
        // not something the compiler folds.
        val horizon = maxOf(HORIZON, 0L)
        assertTrue(2 * (horizon - 1) + 2 > 0)
        assertTrue(2 * horizon + 2 > 0)
        // The stamp of the entry after that is the first to wrap negative.
        assertTrue(2 * (horizon + 1) + 2 < 0)
    }
}
