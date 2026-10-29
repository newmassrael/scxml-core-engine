// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast log kind's generated files (ADR 0062), compiled and used.
//
// sce-build's tests read what the generator writes for a log and look for text
// in it, which shows the text is what the template says and nothing about
// whether the names in it are the names the runtime and the element's codec
// have. Here the file the generator writes for each log is compiled into this
// test over the element class it imports, and the typed `publish` and `read` it
// defines are used on one thread: a rename in the runtime or in the codec, or a
// constant the template computes wrongly, stops the build or fails an
// assertion. The runtime's own properties are BroadcastLogContractTest's and
// BroadcastLogRuntimeTest's; this file holds what the generated file adds.
//
// The files come from tests/forge/resources/broadcast_log_conformance_*.scxml,
// generated as one document set by the generateBroadcastLogFixtures task of
// build.gradle.kts, because a log's slot is sized from its element's document.

package com.sce.forge.runtime.broadcastlog

import com.sce.forge.runtime.CodecError
import com.sce.generated.broadcast_log_conformance_event.BroadcastLogConformanceEvent
import com.sce.generated.broadcast_log_conformance_literal.BroadcastLogConformanceLiteral
import com.sce.generated.broadcast_log_conformance_literal.BroadcastLogConformanceLiteralPublish
import com.sce.generated.broadcast_log_conformance_literal.BroadcastLogConformanceLiteralPublisher
import com.sce.generated.broadcast_log_conformance_literal.BroadcastLogConformanceLiteralRead
import com.sce.generated.broadcast_log_conformance_literal.DECLARED_PROGRESS as LITERAL_DECLARED_PROGRESS
import com.sce.generated.broadcast_log_conformance_literal.MAX_ENCODED_BYTES as LITERAL_MAX_ENCODED_BYTES
import com.sce.generated.broadcast_log_conformance_literal.RETENTION as LITERAL_RETENTION
import com.sce.generated.broadcast_log_conformance_literal.SLOTS as LITERAL_SLOTS
import com.sce.generated.broadcast_log_conformance_literal.WORDS as LITERAL_WORDS
import com.sce.generated.broadcast_log_conformance_literal.newBroadcastLogConformanceLiteral
import com.sce.generated.broadcast_log_conformance_literal.publish as publishEvent
import com.sce.generated.broadcast_log_conformance_literal.read as readEvent
import com.sce.generated.broadcast_log_conformance_recent_records.BroadcastLogConformanceRecentRecords
import com.sce.generated.broadcast_log_conformance_recent_records.BroadcastLogConformanceRecentRecordsPublish
import com.sce.generated.broadcast_log_conformance_recent_records.BroadcastLogConformanceRecentRecordsPublisher
import com.sce.generated.broadcast_log_conformance_recent_records.BroadcastLogConformanceRecentRecordsRead
import com.sce.generated.broadcast_log_conformance_recent_records.DECLARED_PROGRESS as RECENT_DECLARED_PROGRESS
import com.sce.generated.broadcast_log_conformance_recent_records.MAX_ENCODED_BYTES as RECENT_MAX_ENCODED_BYTES
import com.sce.generated.broadcast_log_conformance_recent_records.RETENTION as RECENT_RETENTION
import com.sce.generated.broadcast_log_conformance_recent_records.SLOTS as RECENT_SLOTS
import com.sce.generated.broadcast_log_conformance_recent_records.WORDS as RECENT_WORDS
import com.sce.generated.broadcast_log_conformance_recent_records.newBroadcastLogConformanceRecentRecords
import com.sce.generated.broadcast_log_conformance_recent_records.publish as publishRecord
import com.sce.generated.broadcast_log_conformance_recent_records.read as readRecord
import com.sce.generated.broadcast_log_conformance_record.BroadcastLogConformanceRecord
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

// The generated aliases are the runtime's types: these accept the alias and
// hand back the runtime's type, and do not compile otherwise. They are named
// apart because both aliases are the one type, so overloads would clash.
private fun literalAsRuntime(log: BroadcastLogConformanceLiteral): BroadcastLog = log

private fun recentAsRuntime(log: BroadcastLogConformanceRecentRecords): BroadcastLog = log

private fun literalPublisherAsRuntime(publisher: BroadcastLogConformanceLiteralPublisher): BroadcastLog.Publisher = publisher

/** The element of the literal log: a sensor id and a 16-bit value, three bytes on the wire. */
private fun event(sensor: Int, value: Int) =
    BroadcastLogConformanceEvent(sensor_id = sensor.toUByte(), value = value.toUShort())

/** A record of the other log: a tag and a payload of the bytes given. */
private fun record(tag: Int, vararg payload: Int) =
    BroadcastLogConformanceRecord(tag = tag.toUByte(), payload = ByteArray(payload.size) { payload[it].toByte() })

private fun BroadcastLogConformanceLiteralPublisher.publishedAs(event: BroadcastLogConformanceEvent): Long {
    val published = assertIs<BroadcastLogConformanceLiteralPublish.Published>(publishEvent(this, event))
    return published.seq
}

private fun BroadcastLogConformanceRecentRecordsPublisher.publishedAs(record: BroadcastLogConformanceRecord): Long {
    val published = assertIs<BroadcastLogConformanceRecentRecordsPublish.Published>(publishRecord(this, record))
    return published.seq
}

/** The entry [seq] of the literal log is [element], as the next typed read says. */
private fun assertEventItem(
    log: BroadcastLogConformanceLiteral,
    reader: Reader,
    seq: Long,
    element: BroadcastLogConformanceEvent,
) {
    assertEquals(BroadcastLogConformanceLiteralRead.Item(seq, element), readEvent(log, reader))
}

/** The entry [seq] of the record log is a record with [tag] and [payload], as the next typed read says. */
private fun assertRecordItem(
    log: BroadcastLogConformanceRecentRecords,
    reader: Reader,
    seq: Long,
    tag: Int,
    payload: List<Int>,
) {
    val item = assertIs<BroadcastLogConformanceRecentRecordsRead.Item>(readRecord(log, reader))
    assertEquals(seq, item.seq)
    assertEquals(tag.toUByte(), item.element.tag)
    assertEquals(payload.map { it.toByte() }, item.element.payload.toList(), "the payload is what was published, no longer and no shorter")
}

class BroadcastLogGeneratedTest {
    @Test
    fun theLiteralLogFileStatesWhatTheDocumentRequiredAndWhatItsElementNeeds() {
        assertEquals(3, LITERAL_RETENTION, "the retention is the document's: three")
        assertEquals(LITERAL_RETENTION + 1, LITERAL_SLOTS, "a slot more than the retention")
        // The element is a uint8 at byte 0 and a uint16 at bytes 1 and 2: 1 + 2 = 3 bytes.
        assertEquals(3, LITERAL_MAX_ENCODED_BYTES)
        // 1 length word + ceil(3 / 8) = 1 + 1 = 2 words.
        assertEquals(2, LITERAL_WORDS)
        assertEquals("wait-free", LITERAL_DECLARED_PROGRESS)
        val log = literalAsRuntime(newBroadcastLogConformanceLiteral())
        assertEquals(LITERAL_WORDS, log.words)
        assertEquals(LITERAL_SLOTS, log.slots)
        assertEquals(LITERAL_RETENTION.toLong(), log.retention)
    }

    @Test
    fun theRecordLogFileSizesAnEntryFromTheLargestRecordItsDocumentAllows() {
        assertEquals(2, RECENT_RETENTION, "the retention is the document's: two")
        assertEquals(RECENT_RETENTION + 1, RECENT_SLOTS)
        // A one-byte tag and a payload of at most 8 bytes: 1 + 8 = 9 bytes.
        assertEquals(9, RECENT_MAX_ENCODED_BYTES)
        // 1 length word + ceil(9 / 8) = 1 + 2 = 3 words.
        assertEquals(3, RECENT_WORDS)
        assertEquals("wait-free", RECENT_DECLARED_PROGRESS)
        val log = recentAsRuntime(newBroadcastLogConformanceRecentRecords())
        assertEquals(RECENT_WORDS, log.words)
        assertEquals(RECENT_SLOTS, log.slots)
    }

    @Test
    fun entriesComeBackWholeAndInOrderThroughTheTypedFunctions() {
        val log = newBroadcastLogConformanceLiteral()
        val publisher = literalPublisherAsRuntime(assertNotNull(log.publisher()))
        assertEquals(0L, publisher.publishedAs(event(1, 10)))
        assertEquals(1L, publisher.publishedAs(event(2, 20)))
        assertEquals(2L, publisher.publishedAs(event(3, 30)))

        // Three entries are all the log retains, so the oldest is still the first.
        val reader = log.readerOldest()
        assertEquals(0L, reader.cursor)
        assertEventItem(log, reader, 0, event(1, 10))
        assertEventItem(log, reader, 1, event(2, 20))
        assertEventItem(log, reader, 2, event(3, 30))
        assertEquals(BroadcastLogConformanceLiteralRead.Empty, readEvent(log, reader), "nothing newer is published")
        assertEquals(3L, reader.cursor, "an empty read leaves the cursor at the head")
    }

    @Test
    fun aRecordShorterThanItsBoundIsStoredWithItsOwnLengthAndComesBackAsWritten() {
        val log = newBroadcastLogConformanceRecentRecords()
        val publisher = assertNotNull(log.publisher())
        // Encoded: 1 + 8 = 9 bytes (the bound), 1 + 3 = 4 bytes, and 1 + 0 = 1 byte. The
        // log retains two, so the three go through it one reader at a time.
        val reader = log.readerOldest()
        assertEquals(0L, publisher.publishedAs(record(7, 1, 2, 3, 4, 5, 6, 7, 8)))
        assertRecordItem(log, reader, 0, 7, listOf(1, 2, 3, 4, 5, 6, 7, 8))
        assertEquals(1L, publisher.publishedAs(record(9, 200, 201, 202)))
        assertRecordItem(log, reader, 1, 9, listOf(200, 201, 202))
        assertEquals(2L, publisher.publishedAs(record(11)))
        assertRecordItem(log, reader, 2, 11, emptyList())
        assertEquals(BroadcastLogConformanceRecentRecordsRead.Empty, readRecord(log, reader))
    }

    @Test
    fun aReaderThePublisherLapsIsToldHowManyItMissedAndResumesAtTheOldest() {
        val log = newBroadcastLogConformanceLiteral()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerAt(0)
        for (n in 0 until 6) {
            assertEquals(n.toLong(), publisher.publishedAs(event(1, n * 10)))
        }
        // h = 6 entries published and the retention is 3, so the window is [6 - 3, 6) = [3, 6).
        // The reader holds 0, so it missed 3 - 0 = 3 entries and now holds 3.
        assertEquals(BroadcastLogConformanceLiteralRead.Lagged(3), readEvent(log, reader))
        assertEquals(3L, reader.cursor)
        assertEquals(3L, reader.lost)
        assertEventItem(log, reader, 3, event(1, 30))
        assertEventItem(log, reader, 4, event(1, 40))
        assertEventItem(log, reader, 5, event(1, 50))
        assertEquals(BroadcastLogConformanceLiteralRead.Empty, readEvent(log, reader))
    }

    @Test
    fun aLappedReaderOfTheRecordLogResumesAtItsOldestRecord() {
        val log = newBroadcastLogConformanceRecentRecords()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerAt(0)
        for (n in 0 until 4) {
            assertEquals(n.toLong(), publisher.publishedAs(record(n, n, n + 1)))
        }
        // h = 4 and the retention is 2, so the window is [4 - 2, 4) = [2, 4): the reader at 0 missed 2.
        assertEquals(BroadcastLogConformanceRecentRecordsRead.Lagged(2), readRecord(log, reader))
        assertRecordItem(log, reader, 2, 2, listOf(2, 3))
        assertRecordItem(log, reader, 3, 3, listOf(3, 4))
    }

    @Test
    fun twoReadersKeepCursorsOfTheirOwn() {
        val log = newBroadcastLogConformanceLiteral()
        val publisher = assertNotNull(log.publisher())
        for (n in 0 until 3) {
            publisher.publishedAs(event(2, n))
        }
        val first = log.readerOldest()
        val second = log.readerOldest()
        assertEventItem(log, first, 0, event(2, 0))
        assertEventItem(log, first, 1, event(2, 1))
        // The first reader has read two and the second none: neither moved the other.
        assertEquals(2L, first.cursor)
        assertEquals(0L, second.cursor)
        assertEventItem(log, second, 0, event(2, 0))
        assertEventItem(log, first, 2, event(2, 2))
        assertEquals(3L, first.cursor)
        assertEquals(1L, second.cursor)
        assertEventItem(log, second, 1, event(2, 1))
        assertEventItem(log, second, 2, event(2, 2))
    }

    @Test
    fun aRecordLargerThanItsBoundIsRefusedAndNeverTruncated() {
        val log = newBroadcastLogConformanceRecentRecords()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        // A payload of 9 bytes encodes to 1 + 9 = 10 bytes, one more than the 9 the log holds.
        val refused = assertIs<BroadcastLogConformanceRecentRecordsPublish.Refused>(
            publishRecord(publisher, record(5, 1, 2, 3, 4, 5, 6, 7, 8, 9)),
        )
        assertEquals(CodecError.BufferOverflow, refused.error, "the bounded sink refuses what does not fit")
        assertEquals(0L, log.head(), "a refused element publishes nothing")
        assertEquals(BroadcastLogConformanceRecentRecordsRead.Empty, readRecord(log, reader))
        // The sequence has no gap where the refused element was: the next one is the first.
        assertEquals(0L, publisher.publishedAs(record(5, 1, 2, 3, 4, 5, 6, 7, 8)))
        assertRecordItem(log, reader, 0, 5, listOf(1, 2, 3, 4, 5, 6, 7, 8))
    }

    @Test
    fun aStoredLengthAboveTheBoundIsAnUnreadableEntryAndNeverAnIndexOutOfRange() {
        val log = newBroadcastLogConformanceLiteral()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        // The log holds entries of the typed publish only; these two words are what a
        // copy of another writer's would look like, the length first. WORDS = 2.
        assertEquals(2, LITERAL_WORDS)
        publisher.publish(longArrayOf(1000L, 0L))
        publisher.publish(longArrayOf(-1L, 0L))
        val tooLong = assertIs<BroadcastLogConformanceLiteralRead.Unreadable>(readEvent(log, reader))
        assertEquals(0L, tooLong.seq)
        assertEquals("the stored length 1000 is above the 3 bytes this log holds", tooLong.why)
        // A length is unsigned in the contract, so a word with its top bit set is above the bound too.
        val negative = assertIs<BroadcastLogConformanceLiteralRead.Unreadable>(readEvent(log, reader))
        assertEquals(1L, negative.seq)
        assertEquals("the stored length -1 is above the 3 bytes this log holds", negative.why)
        assertEquals(2L, reader.cursor, "the reader has moved past each unreadable entry")
    }

    @Test
    fun aStoredLengthWithinTheBoundThatDoesNotDecodeIsAnUnreadableEntry() {
        val log = newBroadcastLogConformanceLiteral()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        // The element is three bytes, so two stored bytes are a truncated frame.
        publisher.publish(longArrayOf(2L, 0x0201L))
        val unreadable = assertIs<BroadcastLogConformanceLiteralRead.Unreadable>(readEvent(log, reader))
        assertEquals(0L, unreadable.seq)
        assertTrue(unreadable.why.contains("do not decode"), unreadable.why)
        assertEquals(1L, reader.cursor)
    }
}
