// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

// The Kotlin arm's stress runs of the `queue` kind, written as histories (SCE
// Protocol-Synthesis RFC §synth-5-P, verification layer 2).
//
// These tests run the queues of this package on real threads, record every
// attempt each thread made, and write one history per run in the JSON form
// every backend writes (tests/forge/conformance/queue_history.schema.json).
// They judge nothing about linearizability: `sce-codegen check-queue-history`
// does, for this backend as for the others, so no backend carries a checker of
// its own. The Kotlin gate (scripts/gates/forge-kotlin.sh) sets
// SCE_QUEUE_HISTORY_DIR, runs these, then runs the command over what they
// wrote. Without the variable there is nowhere to write, and the tests are
// skipped by name.
//
// The recording follows the Rust arm's: one counter that every thread reads
// immediately before and immediately after each call, so a read-modify-write
// chain on one atomic orders the readings with the calls between them and
// "returned before invoked" in the record means it in the run. Every attempt is
// recorded, the refused pushes and the empty pops too, because those are
// results the checker must account for.

package com.sce.forge.runtime.queue

import java.io.File
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertTrue
import org.junit.Assume.assumeTrue

/** One observed operation. A pop that found the queue empty names no value. */
private class Operation(
    val call: String,
    val value: Long?,
    val outcome: String,
    val invoked: Long,
    val returned: Long,
) {
    fun toJson(): String {
        val named = if (value == null) "" else ""","value":$value"""
        return """{"call":"$call"$named,"outcome":"$outcome","invoked":$invoked,"returned":$returned}"""
    }
}

private class History(val capacity: Int, val refusal: String, val participants: List<List<Operation>>) {
    fun toJson(): String {
        val logs = participants.joinToString(",") { log -> log.joinToString(",", "[", "]") { it.toJson() } }
        return """{"version":1,"capacity":$capacity,"refusal":"$refusal","participants":[$logs]}"""
    }
}

private const val REFUSAL_AT_CAPACITY = "at-capacity"
private const val REFUSAL_WHILE_SLOTS_ARE_HELD = "while-slots-are-held"

/**
 * Runs of each Lamport capacity are recorded. A Lamport run pushes thousands of
 * values and its history is megabytes, and the ring has no schedule a second
 * run reaches that the first does not.
 */
private const val RUNS_PER_SPSC_CAPACITY = 3

/** Values that go through each Lamport run. */
private const val VALUES_PER_SPSC_RUN = 2000

/**
 * Runs of each SCQ shape are recorded: short runs, many of them, because the
 * schedules that matter are a small fraction of those a machine gives.
 */
private const val RUNS_PER_SCQ_SHAPE = 25

private const val DEADLINE_NANOS = 120_000_000_000L

/** Where the histories are written, or null when nowhere is named. */
private fun historyDir(): File? {
    val named = System.getenv("SCE_QUEUE_HISTORY_DIR")
    assumeTrue("SCE_QUEUE_HISTORY_DIR names where the histories are written; it is unset", !named.isNullOrEmpty())
    val dir = File(named!!)
    assertTrue(dir.isDirectory || dir.mkdirs(), "cannot create $dir")
    return dir
}

private fun write(dir: File, name: String, history: History) {
    File(dir, "$name.json").writeText(history.toJson() + "\n")
}

/** The clock every thread reads before and after each call. */
private class Clock {
    private val ticks = AtomicLong(0)
    fun tick(): Long = ticks.addAndFetch(1)
}

/** What the threads of a run say went wrong; a run that recorded a failure is not written. */
private fun failuresOf(failures: ConcurrentLinkedQueue<String>) =
    failures.joinToString("; ").ifEmpty { "none" }

/** One producer and one consumer on two threads through the Lamport ring. */
private fun recordSpscRun(capacity: Int): History {
    val queue = Spsc<Long>(capacity)
    val producer = checkNotNull(queue.producer()) { "a fresh queue hands out a handle a side" }
    val consumer = checkNotNull(queue.consumer()) { "a fresh queue hands out a handle a side" }
    val clock = Clock()
    val deadline = System.nanoTime() + DEADLINE_NANOS
    val failures = ConcurrentLinkedQueue<String>()
    val pushes = ArrayList<Operation>()
    val pops = ArrayList<Operation>()

    val producing = thread(name = "spsc-producer") {
        for (value in 1L..VALUES_PER_SPSC_RUN) {
            while (true) {
                if (System.nanoTime() > deadline) {
                    failures.add("the elements did not all arrive before the deadline")
                    return@thread
                }
                val invoked = clock.tick()
                val status = producer.tryPush(value)
                val returned = clock.tick()
                if (status == PushStatus.Ok) {
                    pushes.add(Operation("push", value, "pushed", invoked, returned))
                    break
                }
                pushes.add(Operation("push", value, "full", invoked, returned))
                Thread.yield()
            }
        }
    }
    val consuming = thread(name = "spsc-consumer") {
        var delivered = 0
        while (delivered < VALUES_PER_SPSC_RUN) {
            if (System.nanoTime() > deadline) {
                failures.add("the elements did not all arrive before the deadline")
                return@thread
            }
            val invoked = clock.tick()
            val value = consumer.tryPop()
            val returned = clock.tick()
            if (value != null) {
                pops.add(Operation("pop", value, "popped", invoked, returned))
                delivered++
            } else {
                pops.add(Operation("pop", null, "empty", invoked, returned))
                Thread.yield()
            }
        }
    }
    producing.join()
    consuming.join()
    assertTrue(failures.isEmpty(), failuresOf(failures))
    return History(capacity, REFUSAL_AT_CAPACITY, listOf(pushes, pops))
}

/**
 * [producers] producers and [consumers] consumers on real threads through the
 * SCQ queue, each producer pushing [perProducer] distinct values.
 */
private fun recordScqRun(capacity: Int, ringSlots: Int, producers: Int, consumers: Int, perProducer: Long): History {
    val queue = Scq<Long>(capacity, ringSlots)
    val clock = Clock()
    val deadline = System.nanoTime() + DEADLINE_NANOS
    val total = producers * perProducer
    val delivered = AtomicLong(0)
    val failures = ConcurrentLinkedQueue<String>()
    val logs = List(producers + consumers) { ArrayList<Operation>() }

    val threads = ArrayList<Thread>()
    for (p in 0 until producers) {
        val handle = checkNotNull(queue.producer()) { "the ring has a place for every producer of a shape" }
        threads.add(thread(name = "scq-producer-$p") {
            try {
                for (i in 0 until perProducer) {
                    val value = p * perProducer + i + 1
                    while (true) {
                        if (System.nanoTime() > deadline) {
                            failures.add("the elements did not all arrive before the deadline")
                            return@thread
                        }
                        val invoked = clock.tick()
                        val status = handle.tryPush(value)
                        val returned = clock.tick()
                        if (status == PushStatus.Ok) {
                            logs[p].add(Operation("push", value, "pushed", invoked, returned))
                            break
                        }
                        logs[p].add(Operation("push", value, "full", invoked, returned))
                        Thread.yield()
                    }
                }
            } finally {
                handle.release()
            }
        })
    }
    for (c in 0 until consumers) {
        val handle = checkNotNull(queue.consumer()) { "the ring has a place for every consumer of a shape" }
        threads.add(thread(name = "scq-consumer-$c") {
            try {
                val mine = logs[producers + c]
                while (delivered.load() < total) {
                    if (System.nanoTime() > deadline) {
                        failures.add("the elements did not all arrive before the deadline")
                        return@thread
                    }
                    val invoked = clock.tick()
                    val value = handle.tryPop()
                    val returned = clock.tick()
                    if (value != null) {
                        mine.add(Operation("pop", value, "popped", invoked, returned))
                        delivered.addAndFetch(1)
                    } else {
                        mine.add(Operation("pop", null, "empty", invoked, returned))
                        Thread.yield()
                    }
                }
            } finally {
                handle.release()
            }
        })
    }
    threads.forEach { it.join() }
    assertTrue(failures.isEmpty(), failuresOf(failures))
    return History(capacity, REFUSAL_WHILE_SLOTS_ARE_HELD, logs)
}

class QueueHistoryTest {
    @Test
    fun theLamportRingRunsAreWrittenAsHistories() {
        val dir = historyDir()!!
        for (capacity in listOf(1, 2, 3, 8)) {
            for (run in 0 until RUNS_PER_SPSC_CAPACITY) {
                write(dir, "kotlin_spsc_n${capacity}_$run", recordSpscRun(capacity))
            }
        }
    }

    @Test
    fun theScqRunsAreWrittenAsHistories() {
        val dir = historyDir()!!
        // The shapes are the Rust arm's. A ring is at least as large as the
        // number of participants working it, so the small capacities ride on
        // rings sized for their threads.
        class Shape(val capacity: Int, val ringSlots: Int, val producers: Int, val consumers: Int, val perProducer: Long)
        val shapes = listOf(
            Shape(1, 2, 2, 2, 150),
            Shape(3, 4, 2, 2, 150),
            Shape(8, 8, 2, 2, 150),
            Shape(2, 4, 3, 1, 120),
            Shape(4, 4, 1, 3, 120),
            Shape(5, 8, 2, 2, 150),
        )
        for (s in shapes) {
            for (run in 0 until RUNS_PER_SCQ_SHAPE) {
                val history = recordScqRun(s.capacity, s.ringSlots, s.producers, s.consumers, s.perProducer)
                write(dir, "kotlin_scq_n${s.capacity}_r${s.ringSlots}_p${s.producers}_c${s.consumers}_$run", history)
            }
        }
    }
}
