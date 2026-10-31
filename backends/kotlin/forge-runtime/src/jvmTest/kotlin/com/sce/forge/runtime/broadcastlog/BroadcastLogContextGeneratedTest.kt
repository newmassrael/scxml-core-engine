// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast log kind's generated files over elements whose codec asks its caller
// for what its wire form does not carry (ADR 0062 section 4, "Caller inputs are part
// of the element"), compiled and used.
//
// An element's codec may take single-bit flag inputs on `encode` and `decode`, and a
// variant with no tag field takes its tag on `decode`. The log stores them in the
// entry's context words, takes them on `publish` and returns them with the item, so
// every reader decodes an entry under what it was published under. Here the file the
// generator writes for each of the two logs is compiled over the element classes it
// imports, and the typed functions are used on one thread: a rename in the codec, a
// context word in the wrong place or a flag read under the wrong bit stops the build
// or fails an assertion.
//
// The six documents are tests/forge/conformance/broadcast_log_context/*.scxml, shared by
// every backend's conformance lane and generated as one document set by the
// generateBroadcastLogContextFixtures task of build.gradle.kts.

package com.sce.forge.runtime.broadcastlog

import com.sce.forge.runtime.CodecError
import com.sce.generated.broadcast_log_conformance_choice.BroadcastLogConformanceChoice
import com.sce.generated.broadcast_log_conformance_choice.BroadcastLogConformanceChoiceVariant
import com.sce.generated.broadcast_log_conformance_choice_large.BroadcastLogConformanceChoiceLarge
import com.sce.generated.broadcast_log_conformance_choice_small.BroadcastLogConformanceChoiceSmall
import com.sce.generated.broadcast_log_conformance_chosen.BroadcastLogConformanceChosenPublish
import com.sce.generated.broadcast_log_conformance_chosen.BroadcastLogConformanceChosenRead
import com.sce.generated.broadcast_log_conformance_chosen.CONTEXT_WORDS as CHOSEN_CONTEXT_WORDS
import com.sce.generated.broadcast_log_conformance_chosen.MAX_ENCODED_BYTES as CHOSEN_MAX_ENCODED_BYTES
import com.sce.generated.broadcast_log_conformance_chosen.WORDS as CHOSEN_WORDS
import com.sce.generated.broadcast_log_conformance_chosen.newBroadcastLogConformanceChosen
import com.sce.generated.broadcast_log_conformance_chosen.publish as publishChosen
import com.sce.generated.broadcast_log_conformance_chosen.read as readChosen
import com.sce.generated.broadcast_log_conformance_flagged.BroadcastLogConformanceFlaggedFlags
import com.sce.generated.broadcast_log_conformance_flagged.BroadcastLogConformanceFlaggedPublish
import com.sce.generated.broadcast_log_conformance_flagged.BroadcastLogConformanceFlaggedRead
import com.sce.generated.broadcast_log_conformance_flagged.CONTEXT_WORDS as FLAGGED_CONTEXT_WORDS
import com.sce.generated.broadcast_log_conformance_flagged.MAX_ENCODED_BYTES as FLAGGED_MAX_ENCODED_BYTES
import com.sce.generated.broadcast_log_conformance_flagged.WORDS as FLAGGED_WORDS
import com.sce.generated.broadcast_log_conformance_flagged.newBroadcastLogConformanceFlagged
import com.sce.generated.broadcast_log_conformance_flagged.publish as publishFlagged
import com.sce.generated.broadcast_log_conformance_flagged.read as readFlagged
import com.sce.generated.broadcast_log_conformance_flagged_event.BroadcastLogConformanceFlaggedEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull

/** The element of the flagged log: a sensor id and a 16-bit value that is on the wire only under the flag. */
private fun flaggedEvent(sensorId: Int, value: Int?) =
    BroadcastLogConformanceFlaggedEvent(sensor_id = sensorId.toUByte(), value = value?.toUShort())

/** The choice, as its one-byte arm. */
private fun small(payload: Int) = BroadcastLogConformanceChoice(
    body = BroadcastLogConformanceChoiceVariant.BroadcastLogConformanceChoiceSmall(
        BroadcastLogConformanceChoiceSmall(payload = payload.toUByte()),
    ),
)

/** The choice, as its two-byte arm. */
private fun large(payload: Int) = BroadcastLogConformanceChoice(
    body = BroadcastLogConformanceChoiceVariant.BroadcastLogConformanceChoiceLarge(
        BroadcastLogConformanceChoiceLarge(payload = payload.toUShort()),
    ),
)

/** The entry [seq] of the flagged log is [element] under [hasValue], as the next typed read says. */
private fun assertFlaggedItem(
    log: BroadcastLog,
    reader: Reader,
    seq: Long,
    element: BroadcastLogConformanceFlaggedEvent,
    hasValue: Boolean,
) {
    assertEquals(
        BroadcastLogConformanceFlaggedRead.Item(seq, element, BroadcastLogConformanceFlaggedFlags(hasValue = hasValue)),
        readFlagged(log, reader),
        "entry $seq comes back whole, with the flag it was published under",
    )
}

/** The words of the entry the reader's next read copies, as the runtime hands them over. */
private fun rawEntry(log: BroadcastLog, reader: Reader, words: Int): List<Long> {
    val entry = LongArray(words)
    assertIs<Read.Item>(log.read(reader, entry))
    return entry.toList()
}

class BroadcastLogContextGeneratedTest {
    @Test
    fun anEntryHoldsItsContextWordsBetweenTheLengthAndTheBytes() {
        // The flagged element is an 8-bit id and a gated 16-bit value: at most 3 bytes.
        // One length word, one context word (one flag), one word of bytes.
        assertEquals(3, FLAGGED_MAX_ENCODED_BYTES)
        assertEquals(1, FLAGGED_CONTEXT_WORDS)
        assertEquals(3, FLAGGED_WORDS)
        assertEquals(FLAGGED_WORDS, newBroadcastLogConformanceFlagged().words)
        // The choice is the larger of its arms, two bytes; the tag is one context word.
        assertEquals(2, CHOSEN_MAX_ENCODED_BYTES)
        assertEquals(1, CHOSEN_CONTEXT_WORDS)
        assertEquals(3, CHOSEN_WORDS)
        assertEquals(CHOSEN_WORDS, newBroadcastLogConformanceChosen().words)
    }

    @Test
    fun theFlagIsStoredInTheContextWordAndTheBytesFollowIt() {
        val log = newBroadcastLogConformanceFlagged()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        // Under the flag the element is three bytes (the id, then the value little-endian) and
        // the context word is 1; without it the element is the id alone and the word is 0.
        assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
            publishFlagged(publisher, flaggedEvent(1, 0x0203), BroadcastLogConformanceFlaggedFlags(hasValue = true)),
        )
        assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
            publishFlagged(publisher, flaggedEvent(2, null), BroadcastLogConformanceFlaggedFlags(hasValue = false)),
        )
        assertEquals(listOf(3L, 1L, 0x020301L), rawEntry(log, reader, FLAGGED_WORDS))
        assertEquals(listOf(1L, 0L, 0x02L), rawEntry(log, reader, FLAGGED_WORDS))
    }

    @Test
    fun aReaderGetsBackTheFlagAnEntryWasPublishedUnder() {
        val log = newBroadcastLogConformanceFlagged()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()

        // The same element shape, published under each answer of the flag: the value is on the
        // wire under `true` and absent under `false`, so the two frames are three bytes and one,
        // and neither can be decoded under the other's flag.
        val published = listOf(
            flaggedEvent(1, 0x0203) to true,
            flaggedEvent(2, null) to false,
        )
        for ((element, hasValue) in published) {
            assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
                publishFlagged(publisher, element, BroadcastLogConformanceFlaggedFlags(hasValue = hasValue)),
            )
        }
        for ((seq, entry) in published.withIndex()) {
            assertFlaggedItem(log, reader, seq.toLong(), entry.first, entry.second)
        }
        assertEquals(BroadcastLogConformanceFlaggedRead.Empty, readFlagged(log, reader))
    }

    @Test
    fun aFlagItsElementContradictsIsRefusedByTheCodecAndNothingIsPublished() {
        // The codec checks a flag input against the field it gates before it writes a byte
        // (`CodecError.PresentIfMismatch`, as the Rust codec does). The log stores what the
        // codec accepted and judges nothing of its own, so a contradiction never reaches an
        // entry: nothing is published and no sequence number is taken.
        val log = newBroadcastLogConformanceFlagged()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()

        val contradictions = listOf(
            // The value is there and the flag says it is not.
            flaggedEvent(1, 0x0203) to false,
            // The value is absent and the flag says it is there.
            flaggedEvent(2, null) to true,
        )
        for ((element, hasValue) in contradictions) {
            val refused = assertIs<BroadcastLogConformanceFlaggedPublish.Refused>(
                publishFlagged(publisher, element, BroadcastLogConformanceFlaggedFlags(hasValue = hasValue)),
            )
            assertEquals(CodecError.PresentIfMismatch, refused.error)
            assertEquals(0L, log.head(), "a refused flag publishes nothing")
        }
        assertEquals(BroadcastLogConformanceFlaggedRead.Empty, readFlagged(log, reader))

        // An element and a flag that agree are published as the first entry.
        val published = assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
            publishFlagged(publisher, flaggedEvent(1, 0x0203), BroadcastLogConformanceFlaggedFlags(hasValue = true)),
        )
        assertEquals(0L, published.seq, "the refusals took no sequence number")
    }

    @Test
    fun everyReaderOfAnEntryDecodesItUnderTheSameFlag() {
        val log = newBroadcastLogConformanceFlagged()
        val publisher = assertNotNull(log.publisher())
        val early = log.readerOldest()
        val late = log.readerOldest()
        assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
            publishFlagged(publisher, flaggedEvent(5, 500), BroadcastLogConformanceFlaggedFlags(hasValue = true)),
        )
        assertIs<BroadcastLogConformanceFlaggedPublish.Published>(
            publishFlagged(publisher, flaggedEvent(6, null), BroadcastLogConformanceFlaggedFlags(hasValue = false)),
        )
        for (reader in listOf(early, late)) {
            assertFlaggedItem(log, reader, 0, flaggedEvent(5, 500), true)
            assertFlaggedItem(log, reader, 1, flaggedEvent(6, null), false)
        }
    }

    @Test
    fun theTagIsStoredInTheContextWordAndTheArmsBytesFollowIt() {
        val log = newBroadcastLogConformanceChosen()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        // The wire form of an arm carries no tag: the small arm is its one byte and the large arm
        // its two bytes little-endian, and the context word is the tag the publisher gave.
        assertIs<BroadcastLogConformanceChosenPublish.Published>(publishChosen(publisher, small(9), 1.toUByte()))
        assertIs<BroadcastLogConformanceChosenPublish.Published>(publishChosen(publisher, large(0x0102), 0.toUByte()))
        assertEquals(listOf(1L, 1L, 0x09L), rawEntry(log, reader, CHOSEN_WORDS))
        assertEquals(listOf(2L, 0L, 0x0102L), rawEntry(log, reader, CHOSEN_WORDS))
    }

    @Test
    fun aReaderGetsBackTheTagAnEntryWasPublishedUnder() {
        val log = newBroadcastLogConformanceChosen()
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()

        // The wire form of an arm carries no tag, so the two arms' bytes are the arm's own and
        // which arm a frame is comes from the tag stored beside it.
        val sent = listOf(
            small(9) to 1,
            large(0x0102) to 0,
            small(255) to 1,
        )
        for ((element, tag) in sent) {
            assertIs<BroadcastLogConformanceChosenPublish.Published>(publishChosen(publisher, element, tag.toUByte()))
        }
        for ((seq, entry) in sent.withIndex()) {
            assertEquals(
                BroadcastLogConformanceChosenRead.Item(seq.toLong(), entry.first, entry.second.toUByte()),
                readChosen(log, reader),
                "entry $seq comes back as the arm it was published as",
            )
        }
        assertEquals(BroadcastLogConformanceChosenRead.Empty, readChosen(log, reader))
    }
}
