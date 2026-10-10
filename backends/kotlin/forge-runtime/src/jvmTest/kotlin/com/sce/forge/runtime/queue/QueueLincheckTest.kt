// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin queues under Lincheck's model checker: layer 3 of the queue kind's
// verification (SCE Protocol-Synthesis RFC §synth-5-P). The stress runs sample
// the few interleavings a machine happens to give; the model checker drives the
// threads itself and visits the interleavings of every shared access, and
// judges each against a sequential FIFO. It also checks obstruction freedom: an
// operation that runs alone must finish, which no algorithm that waits for
// another thread does.
//
// TWO CLAIMS, and the second is what makes the first worth reading:
//
//   1. Each queue of this package is linearizable against the sequential spec
//      below, and is obstruction-free, over every interleaving the checker
//      visits.
//   2. The checker can tell a broken queue from a sound one. A green result
//      from a checker that never sees a violation says nothing, so a queue
//      that loses an update and a queue that takes a lock are run through the
//      same options and must be refused.
//
// The capacity of the bounded queues is above the most pushes a scenario can
// hold, so a refusal never occurs. That is deliberate and is not a gap: an SCQ
// push may be refused while other operations hold slots (at most one per
// participant), which a sequential FIFO cannot express, and the stress runs'
// history checker judges that refusal where the history says who was in
// flight. This test judges the part a sequential spec can: no element lost,
// duplicated or reordered.

package com.sce.forge.runtime.queue

import java.util.concurrent.locks.ReentrantLock
import kotlin.test.Test
import kotlin.test.assertFailsWith
import org.jetbrains.lincheck.LincheckAssertionError
import org.jetbrains.lincheck.datastructures.ModelCheckingOptions
import org.jetbrains.lincheck.datastructures.Operation

/** Elements a bounded queue under test holds; above the pushes one scenario makes. */
private const val CAPACITY = 16

/**
 * How often an SCQ ring's dequeue looks again at an entry still empty, under the
 * checker. The runtime's own count is 10,000, a spin the checker cannot follow:
 * measured 2026-10-10, it stopped a pop on an EMPTY queue after 51 reads of one
 * entry and reported a hang, in a scenario with one thread. The loop is bounded
 * and ends by moving the entry; the count is patience and not part of the
 * correctness argument, and the Rust runtime's loom build looks once for the same
 * reason. One look still reaches both branches: the re-read and the give-up.
 */
private const val CHECKER_SPINS = 1

/** The scenario every queue is checked with: a prefix, two racing threads, a suffix. */
private fun options(): ModelCheckingOptions = ModelCheckingOptions()
    .iterations(30)
    .invocationsPerIteration(2_000)
    .threads(2)
    .actorsBefore(1)
    .actorsPerThread(3)
    .actorsAfter(1)
    .checkObstructionFreedom(true)

/** A FIFO that refuses a push at [CAPACITY]: what the bounded queues are judged against. */
class BoundedFifo {
    private val items = ArrayDeque<Int>()

    fun push(value: Int): Boolean {
        if (items.size >= CAPACITY) {
            return false
        }
        items.addLast(value)
        return true
    }

    fun pop(): Int? = items.removeFirstOrNull()
}

/** A FIFO that never refuses: what the segmented queues, over an allocator that never refuses, are judged against. */
class UnboundedFifo {
    private val items = ArrayDeque<Int>()

    fun push(value: Int): Boolean {
        items.addLast(value)
        return true
    }

    fun pop(): Int? = items.removeFirstOrNull()
}

/** The SCQ row: any number of producers and consumers. */
class ScqUnderTest {
    private val queue = Scq<Int>(CAPACITY, CAPACITY, CHECKER_SPINS)
    private val producer = checkNotNull(queue.producer())
    private val consumer = checkNotNull(queue.consumer())

    @Operation
    fun push(value: Int): Boolean = producer.tryPush(value) == PushStatus.Ok

    @Operation
    fun pop(): Int? = consumer.tryPop()
}

/** The Lamport row: one producer and one consumer, so each side's operations stay on one thread. */
class SpscUnderTest {
    private val queue = Spsc<Int>(CAPACITY)
    private val producer = checkNotNull(queue.producer())
    private val consumer = checkNotNull(queue.consumer())

    @Operation(nonParallelGroup = "producer")
    fun push(value: Int): Boolean = producer.tryPush(value) == PushStatus.Ok

    @Operation(nonParallelGroup = "consumer")
    fun pop(): Int? = consumer.tryPop()
}

/**
 * The list of SCQ rings. A segment of two elements over four-slot rings, so the
 * few pushes a scenario makes close a ring and link a successor: the hand-over
 * is what the model checker is here for.
 */
class LscqUnderTest {
    private val queue = checkNotNull(Lscq.create<Int>(2, 4, BudgetAllocator(), Progress.LockFree, CHECKER_SPINS))
    private val producer = checkNotNull(queue.producer())
    private val consumer = checkNotNull(queue.consumer())

    @Operation
    fun push(value: Int): Boolean = producer.tryPush(value) == PushStatus.Ok

    @Operation
    fun pop(): Int? = consumer.tryPop()
}

/** The linked Lamport rings: one producer and one consumer, two elements to a segment. */
class LinkedLamportUnderTest {
    private val queue = checkNotNull(LinkedLamport.create<Int>(2, BudgetAllocator(), Progress.LockFree))
    private val producer = checkNotNull(queue.producer())
    private val consumer = checkNotNull(queue.consumer())

    @Operation(nonParallelGroup = "producer")
    fun push(value: Int): Boolean = producer.tryPush(value) == PushStatus.Ok

    @Operation(nonParallelGroup = "consumer")
    fun pop(): Int? = consumer.tryPop()
}

/**
 * NEGATIVE CONTROL: a queue that loses an update. Two pushes read the same
 * `tail`, write the same slot and each add one, so one element is gone. Nothing
 * in the code is atomic, which is the point: the checker must find the
 * interleaving.
 */
class RacyQueue {
    private val slots = IntArray(CAPACITY)
    private var head = 0
    private var tail = 0

    @Operation
    fun push(value: Int): Boolean {
        val at = tail
        slots[at % CAPACITY] = value
        tail = at + 1
        return true
    }

    @Operation
    fun pop(): Int? {
        if (head == tail) {
            return null
        }
        val value = slots[head % CAPACITY]
        head += 1
        return value
    }
}

/**
 * NEGATIVE CONTROL: a queue that is correct and not obstruction-free. Every
 * operation takes one lock, so a thread stopped inside it stops every other:
 * linearizable, and exactly what the obstruction-freedom check exists to refuse.
 */
class LockingQueue {
    private val lock = ReentrantLock()
    private val items = ArrayDeque<Int>()

    @Operation
    fun push(value: Int): Boolean {
        lock.lock()
        try {
            items.addLast(value)
        } finally {
            lock.unlock()
        }
        return true
    }

    @Operation
    fun pop(): Int? {
        lock.lock()
        try {
            return items.removeFirstOrNull()
        } finally {
            lock.unlock()
        }
    }
}

class QueueLincheckTest {
    @Test
    fun scqIsLinearizableAndObstructionFree() {
        options().sequentialSpecification(BoundedFifo::class.java).check(ScqUnderTest::class)
    }

    @Test
    fun spscIsLinearizableAndObstructionFree() {
        options().sequentialSpecification(BoundedFifo::class.java).check(SpscUnderTest::class)
    }

    @Test
    fun lscqIsLinearizableAndObstructionFree() {
        options().sequentialSpecification(UnboundedFifo::class.java).check(LscqUnderTest::class)
    }

    @Test
    fun linkedLamportIsLinearizableAndObstructionFree() {
        options().sequentialSpecification(UnboundedFifo::class.java).check(LinkedLamportUnderTest::class)
    }

    @Test
    fun aQueueThatLosesAnUpdateIsRefused() {
        assertFailsWith<LincheckAssertionError> {
            options().sequentialSpecification(UnboundedFifo::class.java).check(RacyQueue::class)
        }
    }

    @Test
    fun aQueueThatTakesALockIsNotObstructionFree() {
        assertFailsWith<LincheckAssertionError> {
            options().sequentialSpecification(UnboundedFifo::class.java).check(LockingQueue::class)
        }
    }
}
