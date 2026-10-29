// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin broadcast log under Lincheck's model checker (RFC §synth-5-S; ADR 0062
// section 6, layer 2). The stress runs sample the few interleavings a machine happens to
// give; the model checker drives the threads itself, visits the interleavings
// of every shared access, and judges each against the sequential log of ADR
// 0062 section 2. It also checks obstruction freedom: an operation that runs
// alone must finish.
//
// TWO CLAIMS, and the second is what makes the first worth reading:
//
//   1. The log is linearizable against the sequential specification below, and
//      is obstruction-free, over every interleaving the checker visits. The
//      specification is the abstract log and has no slots, stamps or fences, so
//      a pass says the ring implements it, torn copies included: a read is
//      compared with its words, and a copy of two entries matches no entry.
//   2. The checker can tell a broken log from a sound one. A log that makes its
//      head visible before it has written the entry is run through the same
//      options and must be refused.
//
// The scenario has the one publisher on one thread and two readers on one
// thread each, so a reader's cursor, which is plain state, is never shared. The
// retention is two, which three publishes pass, so the scenario laps the ring
// and reaches the reload of the head that answers a lag found late. This
// checker reaches the interleavings of a sequentially consistent execution
// (Lincheck does not model weaker orders), which is the strength the Kotlin
// atomics have; the weaker orders of the Rust row are the loom models' to check.

package com.sce.forge.runtime.broadcastlog

import kotlin.test.Test
import kotlin.test.assertFailsWith
import org.jetbrains.lincheck.LincheckAssertionError
import org.jetbrains.lincheck.datastructures.ModelCheckingOptions
import org.jetbrains.lincheck.datastructures.Operation

/** Entries a reader may rely on; the log under test has one slot more. */
private const val RETAINED = 2

/** Words in an entry. Two, so a copy can be torn between them. */
private const val ENTRY_WORDS = 2

/** The scenario every log is checked with: a prefix, three racing threads, a suffix. */
private fun lincheckOptions(): ModelCheckingOptions = ModelCheckingOptions()
    .iterations(30)
    .invocationsPerIteration(2_000)
    .threads(3)
    .actorsBefore(1)
    .actorsPerThread(2)
    .actorsAfter(1)
    .checkObstructionFreedom(true)

/** What a read answered, in a form two runs compare with `equals`. */
private fun describe(result: Read, out: LongArray): String = when (result) {
    is Read.Item -> "item ${result.seq} ${out[0]} ${out[1]}"
    is Read.Lagged -> "lagged ${result.skipped}"
    Read.Empty -> "empty"
}

/**
 * The sequential specification (ADR 0062 section 2): `h` entries published,
 * only the last [RETAINED] retained, and a cursor for each of two readers, both
 * opened at the oldest retained entry of the empty log.
 */
class BroadcastLogSpecification {
    private val values = ArrayList<Int>()
    private val cursors = LongArray(2)

    fun publish(value: Int): Long {
        values.add(value)
        return values.size - 1L
    }

    fun readA(): String = readFor(0)

    fun readB(): String = readFor(1)

    fun head(): Long = values.size.toLong()

    private fun readFor(reader: Int): String {
        val h = values.size.toLong()
        val r = cursors[reader]
        return when {
            r == h -> "empty"
            r < h - RETAINED -> {
                val oldest = h - RETAINED
                cursors[reader] = oldest
                "lagged ${oldest - r}"
            }
            else -> {
                cursors[reader] = r + 1
                val value = values[r.toInt()]
                "item $r $value $value"
            }
        }
    }
}

/** The log of this package: one publisher, two readers, each side on its own thread. */
class BroadcastLogUnderTest {
    private val log = BroadcastLog(ENTRY_WORDS, RETAINED + 1)
    private val publisher = checkNotNull(log.publisher())
    private val readerA = log.readerOldest()
    private val readerB = log.readerOldest()
    private val bufferA = LongArray(ENTRY_WORDS)
    private val bufferB = LongArray(ENTRY_WORDS)

    @Operation(nonParallelGroup = "publisher")
    fun publish(value: Int): Long = publisher.publish(longArrayOf(value.toLong(), value.toLong()))

    @Operation(nonParallelGroup = "readerA")
    fun readA(): String = describe(log.read(readerA, bufferA), bufferA)

    @Operation(nonParallelGroup = "readerB")
    fun readB(): String = describe(log.read(readerB, bufferB), bufferB)

    @Operation
    fun head(): Long = log.head()
}

/**
 * NEGATIVE CONTROL: a log that publishes before it has written. `head` moves
 * past the entry first and the two words are stored afterwards, plain and
 * unguarded, and the reader trusts what it finds. A read that lands between the
 * two steps returns an entry whose words are the previous lap's or are not yet
 * written, which no run of the specification does; the checker must find the
 * interleaving.
 */
class EarlyHeadLog {
    private val slots = RETAINED + 1
    private var head = 0L
    private val first = LongArray(slots)
    private val second = LongArray(slots)
    private val cursors = LongArray(2)

    @Operation(nonParallelGroup = "publisher")
    fun publish(value: Int): Long {
        val n = head
        head = n + 1
        val at = (n % slots).toInt()
        first[at] = value.toLong()
        second[at] = value.toLong()
        return n
    }

    @Operation(nonParallelGroup = "readerA")
    fun readA(): String = readFor(0)

    @Operation(nonParallelGroup = "readerB")
    fun readB(): String = readFor(1)

    @Operation
    fun head(): Long = head

    private fun readFor(reader: Int): String {
        val h = head
        val r = cursors[reader]
        if (r == h) {
            return "empty"
        }
        if (h - r >= slots) {
            val oldest = h - RETAINED
            cursors[reader] = oldest
            return "lagged ${oldest - r}"
        }
        cursors[reader] = r + 1
        val at = (r % slots).toInt()
        return "item $r ${first[at]} ${second[at]}"
    }
}

class BroadcastLogLincheckTest {
    @Test
    fun theLogIsLinearizableAndObstructionFree() {
        lincheckOptions().sequentialSpecification(BroadcastLogSpecification::class.java).check(BroadcastLogUnderTest::class)
    }

    @Test
    fun aLogThatPublishesBeforeItHasWrittenIsRefused() {
        assertFailsWith<LincheckAssertionError> {
            lincheckOptions().sequentialSpecification(BroadcastLogSpecification::class.java).check(EarlyHeadLog::class)
        }
    }
}
