// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

// The Kotlin arm's stress runs of the `broadcast-log` kind, written as histories
// (ADR 0062 section 5 and section 6, layer 3).
//
// These tests run the log of this package on real threads, record every call the
// publisher and each reader made, and write one history per run in the JSON form
// every backend writes (tests/forge/conformance/broadcast_history.schema.json).
// They judge nothing about linearizability: `sce-codegen check-broadcast-history`
// does, for this backend as for the others, so no backend carries a checker of
// its own. The Kotlin gate (scripts/gates/forge-kotlin.sh) sets
// SCE_BROADCAST_HISTORY_DIR, runs these, then runs the command over what they
// wrote. Without the variable there is nowhere to write, and the tests are
// skipped by name.
//
// The recording follows the queue arm's: one counter that every thread reads
// immediately before and immediately after each call, so a read-modify-write
// chain on one atomic orders the readings with the calls between them and
// "returned before invoked" in the record means it in the run. Every call is
// recorded, the empty reads and the lagged ones too, because those are results
// the checker must account for.
//
// The values the publisher publishes are a function of the sequence number, and
// the words of an entry are a function of its value, so a copy that mixes two
// entries fails the run here, before the checker sees it, as well as showing in
// the history as a value no publish wrote. The runs are short: the general
// search of the checker is exponential in the number of the readers' operations
// in the worst case (ADR 0062 section "What step 1 landed"), so each reader's
// record is capped.

package com.sce.forge.runtime.broadcastlog

import java.io.File
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertTrue
import org.junit.Assume.assumeTrue

/** Words in an entry; more than one, so a copy can be torn between them. */
private const val HISTORY_ENTRY_WORDS = 3

/** The publisher reads the head after every this many publishes. */
private const val HEAD_EVERY = 6

/** A reader reads the head after every this many reads. */
private const val READER_HEAD_EVERY = 7

/**
 * Operations a reader records at most, its open and its reads together. A
 * reader that has not reached the end of the log by then stops, and its history
 * is a prefix of what it would have done, which is a history all the same.
 */
private const val MAX_READER_OPERATIONS = 30

/** Where the reader opened `at` asks to start; past the head when the log is young, which clamps. */
private const val OPEN_AT = 3L

/** Runs of each shape that are recorded: short runs, many of them, as the queue arm's. */
private const val RUNS_PER_SHAPE = 25

private const val HISTORY_DEADLINE_NANOS = 120_000_000_000L

/** The value the publish of sequence number [seq] carries. */
private fun valueOf(seq: Long): Long = seq + 100

/** The entry of value [value]: each word is the value and a function of its place. */
private fun entryOfValue(value: Long): LongArray = LongArray(HISTORY_ENTRY_WORDS) { k -> value + k * 1_000L }

/** One observed operation, as the JSON the checker reads. */
private class Observed(
    private val call: String,
    private val fields: String,
    private val invoked: Long,
    private val returned: Long,
) {
    fun toJson(): String = "{\"call\":\"" + call + "\"" + fields + ",\"invoked\":" + invoked + ",\"returned\":" + returned + "}"
}

private class BroadcastHistory(
    val retention: Int,
    val publisher: List<Observed>,
    val readers: List<List<Observed>>,
) {
    fun toJson(): String {
        val published = publisher.joinToString(",", "[", "]") { it.toJson() }
        val read = readers.joinToString(",") { ops -> ops.joinToString(",", "[", "]") { it.toJson() } }
        return "{\"version\":1,\"retention\":" + retention + ",\"publisher\":" + published + ",\"readers\":[" + read + "]}"
    }
}

/** The clock every thread reads before and after each call. */
private class SharedClock {
    private val ticks = AtomicLong(0)
    fun tick(): Long = ticks.addAndFetch(1)
}

/** Where the histories are written, or null when nowhere is named. */
private fun broadcastHistoryDir(): File? {
    val named = System.getenv("SCE_BROADCAST_HISTORY_DIR")
    assumeTrue(
        "SCE_BROADCAST_HISTORY_DIR names where the histories are written; it is unset",
        !named.isNullOrEmpty(),
    )
    val dir = File(named!!)
    assertTrue(dir.isDirectory || dir.mkdirs(), "cannot create $dir")
    return dir
}

private fun writeBroadcastHistory(dir: File, name: String, history: BroadcastHistory) {
    File(dir, "$name.json").writeText(history.toJson() + "\n")
}

/** The head a call returned, as the checker reads it. */
private fun headObservation(log: BroadcastLog, clock: SharedClock): Observed {
    val invoked = clock.tick()
    val head = log.head()
    val returned = clock.tick()
    return Observed("head", ",\"head\":$head", invoked, returned)
}

/**
 * One publisher publishing [total] entries and [readerCount] readers on real
 * threads through a log of [retention] retained entries. The readers open in
 * turn at the oldest entry, at the head and at [OPEN_AT]; each reads until the
 * publisher is finished and it has met the end, or until its record is full.
 */
private fun recordBroadcastRun(retention: Int, readerCount: Int, total: Long): BroadcastHistory {
    val log = BroadcastLog(HISTORY_ENTRY_WORDS, retention + 1)
    val publisher = checkNotNull(log.publisher()) { "a fresh log hands out its publisher" }
    val clock = SharedClock()
    val finished = AtomicBoolean(false)
    val started = CountDownLatch(1)
    val failures = ConcurrentLinkedQueue<String>()
    val deadline = System.nanoTime() + HISTORY_DEADLINE_NANOS
    val publishedOps = ArrayList<Observed>()
    val readerOps = List(readerCount) { ArrayList<Observed>() }

    val threads = ArrayList<Thread>()
    threads.add(thread(name = "broadcast-publisher") {
        try {
            started.countDown()
            for (seq in 0L until total) {
                val value = valueOf(seq)
                val entry = entryOfValue(value)
                val invoked = clock.tick()
                val taken = publisher.publish(entry)
                val returned = clock.tick()
                publishedOps.add(Observed("publish", ",\"value\":$value,\"seq\":$taken", invoked, returned))
                if (taken != seq) {
                    failures.add("publish returned $taken, wanted $seq")
                    return@thread
                }
                if (seq % HEAD_EVERY == HEAD_EVERY - 1L) {
                    publishedOps.add(headObservation(log, clock))
                }
                if (seq % 3L == 0L) {
                    Thread.yield()
                }
            }
        } finally {
            finished.store(true)
            publisher.release()
        }
    })
    for (index in 0 until readerCount) {
        threads.add(thread(name = "broadcast-reader-$index") {
            val ops = readerOps[index]
            val out = LongArray(HISTORY_ENTRY_WORDS)
            started.await()
            repeat(index * 100) { Thread.onSpinWait() }

            val openInvoked = clock.tick()
            val reader: Reader
            val opened: String
            when (index % 3) {
                0 -> {
                    reader = log.readerOldest()
                    opened = ",\"from\":\"oldest\""
                }
                1 -> {
                    reader = log.readerHead()
                    opened = ",\"from\":\"head\""
                }
                else -> {
                    reader = log.readerAt(OPEN_AT)
                    opened = ",\"from\":\"at\",\"at\":$OPEN_AT"
                }
            }
            val openReturned = clock.tick()
            ops.add(Observed("open", opened + ",\"cursor\":" + reader.cursor, openInvoked, openReturned))

            var last = -1L
            var reads = 0
            while (ops.size < MAX_READER_OPERATIONS) {
                if (System.nanoTime() > deadline) {
                    failures.add("reader $index did not reach the end before the deadline")
                    return@thread
                }
                // Taken before the read: if the publisher had finished by then, an
                // empty answer is the end of the log and not a pause.
                val wasFinished = finished.load()
                val before = reader.cursor
                val invoked = clock.tick()
                val result = log.read(reader, out)
                val returned = clock.tick()
                reads++
                when (result) {
                    is Read.Item -> {
                        if (result.seq != before || result.seq <= last) {
                            failures.add("reader $index: item ${result.seq} at cursor $before after $last")
                            return@thread
                        }
                        last = result.seq
                        for (k in 0 until HISTORY_ENTRY_WORDS) {
                            if (out[k] != out[0] + k * 1_000L) {
                                failures.add("reader $index: item ${result.seq} is torn: ${out.toList()}")
                                return@thread
                            }
                        }
                        ops.add(
                            Observed("read", ",\"outcome\":\"item\",\"seq\":${result.seq},\"value\":${out[0]}", invoked, returned),
                        )
                    }
                    is Read.Lagged -> {
                        ops.add(Observed("read", ",\"outcome\":\"lagged\",\"skipped\":${result.skipped}", invoked, returned))
                    }
                    Read.Empty -> {
                        ops.add(Observed("read", ",\"outcome\":\"empty\"", invoked, returned))
                        if (wasFinished) {
                            return@thread
                        }
                        Thread.yield()
                    }
                }
                if (reads % READER_HEAD_EVERY == 0 && ops.size < MAX_READER_OPERATIONS) {
                    ops.add(headObservation(log, clock))
                }
            }
        })
    }
    threads.forEach { it.join() }
    assertTrue(failures.isEmpty(), failures.take(5).joinToString("; ").ifEmpty { "none" })
    return BroadcastHistory(retention, publishedOps, readerOps)
}

class BroadcastLogHistoryTest {
    @Test
    fun theBroadcastLogRunsAreWrittenAsHistories() {
        val dir = broadcastHistoryDir()!!
        // The retentions run from the smallest log, in which every publish
        // after the first laps a reader, to one a reader can follow.
        class Shape(val retention: Int, val readers: Int, val total: Long)
        val shapes = listOf(
            Shape(1, 2, 24),
            Shape(2, 2, 30),
            Shape(2, 3, 24),
            Shape(3, 3, 30),
        )
        for (s in shapes) {
            for (run in 0 until RUNS_PER_SHAPE) {
                val history = recordBroadcastRun(s.retention, s.readers, s.total)
                writeBroadcastHistory(dir, "kotlin_broadcast_r${s.retention}_c${s.readers}_p${s.total}_$run", history)
            }
        }
    }
}
