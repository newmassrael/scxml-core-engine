// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

// The properties of the Kotlin broadcast log that one thread cannot show (ADR
// 0062 sections 4 and 5): with one publisher and several readers on real
// threads, every entry a reader is handed is a whole entry, the sequence
// numbers a reader sees strictly increase, and what a reader was handed plus
// what it was told it missed is exactly what was published.

package com.sce.forge.runtime.broadcastlog

import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

private const val STRESS_DEADLINE_NANOS = 120_000_000_000L

/**
 * The word [k] of the entry [seq]. Every word is a function of the sequence
 * number and of its own position, so a copy that mixes two entries, or a word
 * of one entry in the place of another, disagrees with the entry it claims to be.
 */
private fun wordOf(seq: Long, k: Int): Long = seq * 1_000_003L + k * 7L + 11L

private fun stressEntry(seq: Long, words: Int) = LongArray(words) { k -> wordOf(seq, k) }

/** One reader's account of a run. */
private class ReaderTally(var items: Long = 0, var lagReports: Long = 0)

class BroadcastLogRuntimeTest {
    /**
     * One publisher publishing [total] entries and [readerCount] readers, each
     * opened at the first entry, reading until the publisher is finished and they
     * have reached the end.
     */
    private fun runShape(words: Int, slots: Int, readerCount: Int, total: Long) {
        val log = BroadcastLog(words, slots)
        val publisher = assertNotNull(log.publisher())
        val readers = List(readerCount) { log.readerAt(0) }
        val tallies = List(readerCount) { ReaderTally() }
        val finished = AtomicBoolean(false)
        val start = CountDownLatch(1)
        val failures = ConcurrentLinkedQueue<String>()
        val deadline = System.nanoTime() + STRESS_DEADLINE_NANOS
        val threads = ArrayList<Thread>()

        threads.add(thread(name = "publisher") {
            try {
                start.await()
                for (seq in 0L until total) {
                    val taken = publisher.publish(stressEntry(seq, words))
                    if (taken != seq) {
                        failures.add("publish returned $taken, wanted $seq")
                        return@thread
                    }
                    // Give the readers a turn now and then, so a run is not the
                    // publisher alone followed by the readers alone.
                    if (seq % 61L == 0L) {
                        Thread.yield()
                    }
                }
            } finally {
                finished.store(true)
                publisher.release()
            }
        })
        for (index in 0 until readerCount) {
            threads.add(thread(name = "reader-$index") {
                val reader = readers[index]
                val tally = tallies[index]
                val out = LongArray(words)
                var last = -1L
                while (true) {
                    if (System.nanoTime() > deadline) {
                        failures.add("reader $index did not reach the end before the deadline")
                        return@thread
                    }
                    // Read before the read: if the publisher had finished by then,
                    // an empty answer is the end of the log and not a pause.
                    val wasFinished = finished.load()
                    val before = reader.cursor
                    when (val result = log.read(reader, out)) {
                        is Read.Item -> {
                            tally.items++
                            if (result.seq != before) {
                                failures.add("reader $index: item ${result.seq} at cursor $before")
                                return@thread
                            }
                            if (result.seq <= last) {
                                failures.add("reader $index: item ${result.seq} after $last")
                                return@thread
                            }
                            last = result.seq
                            for (k in 0 until words) {
                                if (out[k] != wordOf(result.seq, k)) {
                                    failures.add("reader $index: item ${result.seq} word $k is ${out[k]}, torn")
                                    return@thread
                                }
                            }
                            if (reader.cursor != result.seq + 1) {
                                failures.add("reader $index: cursor ${reader.cursor} after item ${result.seq}")
                                return@thread
                            }
                        }
                        is Read.Lagged -> {
                            tally.lagReports++
                            if (result.skipped < 1) {
                                failures.add("reader $index: a lag that skipped ${result.skipped}")
                                return@thread
                            }
                            if (reader.cursor != before + result.skipped) {
                                failures.add("reader $index: cursor ${reader.cursor} after skipping ${result.skipped} from $before")
                                return@thread
                            }
                        }
                        Read.Empty -> {
                            if (reader.cursor != before) {
                                failures.add("reader $index: an empty read moved the cursor")
                                return@thread
                            }
                            if (wasFinished) {
                                return@thread
                            }
                            Thread.yield()
                        }
                    }
                }
            })
        }
        start.countDown()
        threads.forEach { it.join() }
        assertTrue(failures.isEmpty(), failures.take(5).joinToString("; "))
        for (index in 0 until readerCount) {
            val reader = readers[index]
            val tally = tallies[index]
            assertEquals(total, reader.cursor, "reader $index ended at the head")
            assertEquals(total, tally.items + reader.lost, "reader $index was handed or told of every entry once")
            assertTrue(tally.items >= 1, "reader $index was handed at least the last entry")
        }
        assertEquals(total, log.head())
    }

    @Test
    fun readersOfASmallRingNeverSeeATornEntry() {
        runShape(words = 4, slots = 2, readerCount = 3, total = 150_000)
    }

    @Test
    fun readersOfAThreeSlotLogNeverSeeATornEntry() {
        runShape(words = 3, slots = 3, readerCount = 3, total = 150_000)
    }

    @Test
    fun readersOfAWideEntryNeverSeeATornEntry() {
        runShape(words = 16, slots = 5, readerCount = 2, total = 100_000)
    }

    @Test
    fun aReaderThatKeepsUpLosesNothing() {
        // The log is larger than the run, so no reader can fall out of the window.
        val total = 500L
        val log = BroadcastLog(2, (total + 1).toInt())
        val publisher = assertNotNull(log.publisher())
        val reader = log.readerOldest()
        val out = LongArray(2)
        for (seq in 0L until total) {
            publisher.publish(stressEntry(seq, 2))
            assertEquals(Read.Item(seq), log.read(reader, out))
            assertEquals(stressEntry(seq, 2).toList(), out.toList())
        }
        assertEquals(0L, reader.lost)
    }
}
