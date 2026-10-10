// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

// The properties of the Kotlin queue runtime that the contract scenarios do not
// reach (SCE Protocol-Synthesis RFC §synth-5-P): wrapping over many laps, the
// places of each side, the refusals of a constructor, the slot a pop clears,
// and that real threads lose nothing and reorder nothing between them.

package com.sce.forge.runtime.queue

import java.lang.ref.WeakReference
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

internal class Payload(val value: Long)

/** Whether [reference] is cleared once the collector has been asked a few times. */
internal fun collected(reference: WeakReference<Payload>): Boolean {
    repeat(50) {
        System.gc()
        if (reference.get() == null) {
            return true
        }
        Thread.sleep(20)
    }
    return false
}

/**
 * Pushes a payload through [push] and returns only a weak reference to it. A
 * function of its own and not a block, so no local of the caller holds the
 * payload and only the queue could.
 */
internal fun pushedPayload(value: Long, push: (Payload) -> PushStatus): WeakReference<Payload> {
    val payload = Payload(value)
    assertEquals(PushStatus.Ok, push(payload))
    return WeakReference(payload)
}

internal const val DEADLINE_NANOS = 120_000_000_000L

class QueueRuntimeTest {
    @Test
    fun aLamportRingKeepsOrderAcrossManyLaps() {
        val queue = Spsc<Long>(3)
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        var next = 0L
        var expected = 0L
        // Far more elements than the lap-index space, so every index wraps.
        repeat(1000) {
            while (producer.tryPush(next) == PushStatus.Ok) {
                next++
            }
            while (true) {
                val popped = consumer.tryPop() ?: break
                assertEquals(expected, popped)
                expected++
            }
        }
        assertEquals(next, expected)
        assertTrue(expected >= 3000, "the run went round the ring many times")
    }

    @Test
    fun aLamportRingRefusesAtCapacityAndKeepsWhatItHolds() {
        val queue = Spsc<Long>(2)
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        assertEquals(PushStatus.Ok, producer.tryPush(1))
        assertEquals(PushStatus.Ok, producer.tryPush(2))
        assertEquals(PushStatus.Full, producer.tryPush(3))
        assertEquals(1L, consumer.tryPop())
        assertEquals(PushStatus.Ok, producer.tryPush(3))
        assertEquals(2L, consumer.tryPop())
        assertEquals(3L, consumer.tryPop())
        assertNull(consumer.tryPop())
    }

    @Test
    fun aLamportRingHasOnePlaceASide() {
        val queue = Spsc<Long>(2)
        val producer = assertNotNull(queue.producer())
        assertNull(queue.producer(), "a second producer is not this algorithm")
        val consumer = assertNotNull(queue.consumer())
        assertNull(queue.consumer(), "a second consumer is not this algorithm")
        producer.release()
        consumer.release()
        assertNotNull(queue.producer(), "a released place is free again")
        assertNotNull(queue.consumer(), "a released place is free again")
    }

    @Test
    fun aNewLamportHandleContinuesWhereTheLastStopped() {
        val queue = Spsc<Long>(2)
        val first = assertNotNull(queue.producer())
        assertEquals(PushStatus.Ok, first.tryPush(1))
        assertEquals(PushStatus.Ok, first.tryPush(2))
        first.release()
        val second = assertNotNull(queue.producer())
        assertEquals(PushStatus.Full, second.tryPush(3), "the ring still holds what the first handle put in")
        val consumer = assertNotNull(queue.consumer())
        assertEquals(1L, consumer.tryPop())
        consumer.release()
        val again = assertNotNull(queue.consumer())
        assertEquals(2L, again.tryPop())
        assertEquals(PushStatus.Ok, second.tryPush(3))
        assertEquals(3L, again.tryPop())
    }

    @Test
    fun aReleasedHandleIsNotUsable() {
        val queue = Spsc<Long>(2)
        val producer = assertNotNull(queue.producer())
        producer.release()
        producer.release()
        assertFailsWith<IllegalStateException> { producer.tryPush(1) }
    }

    @Test
    fun aLamportRingClearsAPoppedSlot() {
        val queue = Spsc<Payload>(2)
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val weak = pushedPayload(7) { producer.tryPush(it) }
        assertEquals(7L, consumer.tryPop()?.value)
        assertTrue(collected(weak), "the queue kept a reference to an element that has left it")
    }

    @Test
    fun aLamportRingLosesNothingBetweenThreads() {
        val queue = Spsc<Long>(4)
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val total = 50_000L
        val failures = ConcurrentLinkedQueue<String>()
        val deadline = System.nanoTime() + DEADLINE_NANOS
        val producing = thread {
            for (value in 0 until total) {
                while (producer.tryPush(value) != PushStatus.Ok) {
                    if (System.nanoTime() > deadline) {
                        failures.add("the elements did not all arrive before the deadline")
                        return@thread
                    }
                    Thread.yield()
                }
            }
        }
        val consuming = thread {
            var expected = 0L
            while (expected < total) {
                if (System.nanoTime() > deadline) {
                    failures.add("the elements did not all arrive before the deadline")
                    return@thread
                }
                val popped = consumer.tryPop()
                if (popped == null) {
                    Thread.yield()
                } else if (popped != expected) {
                    failures.add("popped $popped, wanted $expected")
                    return@thread
                } else {
                    expected++
                }
            }
        }
        producing.join()
        consuming.join()
        assertTrue(failures.isEmpty(), failures.joinToString("; "))
    }

    @Test
    fun anScqQueueFillsAndDrainsAcrossManyCycles() {
        for (capacity in listOf(1, 2, 3, 5, 8)) {
            val queue = Scq<Long>(capacity, ceilPow2(capacity))
            val producer = assertNotNull(queue.producer())
            val consumer = assertNotNull(queue.consumer())
            var next = 0L
            var expected = 0L
            repeat(200) {
                var accepted = 0
                while (producer.tryPush(next) == PushStatus.Ok) {
                    next++
                    accepted++
                }
                assertEquals(capacity, accepted, "exactly the capacity fits when nothing else is running")
                while (true) {
                    val popped = consumer.tryPop() ?: break
                    assertEquals(expected, popped)
                    expected++
                }
            }
            assertEquals(next, expected)
        }
    }

    @Test
    fun anScqSideHandsOutNoMoreHandlesThanItHasPlaces() {
        val queue = Scq<Long>(2, 4)
        val producers = List(4) { assertNotNull(queue.producer()) }
        assertNull(queue.producer(), "a ring cannot be shared by more enqueuers than it has slots")
        assertNull(producers[0].tryClone())
        producers[0].release()
        val clone = assertNotNull(producers[1].tryClone(), "a released place is free again")
        clone.release()
        val consumers = List(4) { assertNotNull(queue.consumer()) }
        assertNull(queue.consumer())
        assertNull(consumers[0].tryClone())
    }

    @Test
    fun anScqQueueRefusesAShapeItCannotRun() {
        assertFailsWith<IllegalArgumentException> { Scq<Long>(0, 2) }
        assertFailsWith<IllegalArgumentException> { Scq<Long>(2, 3) }
        assertFailsWith<IllegalArgumentException> { Scq<Long>(4, 2) }
        assertFailsWith<IllegalArgumentException> { Spsc<Long>(0) }
    }

    @Test
    fun anScqRingClearsAPoppedSlot() {
        val queue = Scq<Payload>(2, 2)
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val weak = pushedPayload(9) { producer.tryPush(it) }
        assertEquals(9L, consumer.tryPop()?.value)
        assertTrue(collected(weak), "the queue kept a reference to an element that has left it")
    }

    @Test
    fun theScqQueueLosesNothingAndReordersNothingBetweenThreads() {
        val producers = 3
        val consumers = 3
        val perProducer = 20_000L
        val queue = Scq<Long>(8, 8)
        val total = producers * perProducer
        val delivered = AtomicLong(0)
        val failures = ConcurrentLinkedQueue<String>()
        val deadline = System.nanoTime() + DEADLINE_NANOS
        // Each consumer keeps the last value it saw from each producer: one
        // producer's elements reach one consumer in the order they were pushed.
        val seenByConsumer = List(consumers) { LongArray(producers) { -1 } }
        val counts = List(consumers) { LongArray(producers) }
        val threads = ArrayList<Thread>()
        for (p in 0 until producers) {
            val handle = assertNotNull(queue.producer())
            threads.add(thread {
                for (i in 0 until perProducer) {
                    while (handle.tryPush(p * perProducer + i) != PushStatus.Ok) {
                        if (System.nanoTime() > deadline) {
                            failures.add("the elements did not all arrive before the deadline")
                            handle.release()
                            return@thread
                        }
                        Thread.yield()
                    }
                }
                handle.release()
            })
        }
        for (c in 0 until consumers) {
            val handle = assertNotNull(queue.consumer())
            threads.add(thread {
                while (delivered.load() < total) {
                    if (System.nanoTime() > deadline) {
                        failures.add("the elements did not all arrive before the deadline")
                        break
                    }
                    val popped = handle.tryPop()
                    if (popped == null) {
                        Thread.yield()
                        continue
                    }
                    val producer = (popped / perProducer).toInt()
                    if (popped <= seenByConsumer[c][producer]) {
                        failures.add("consumer $c saw $popped after ${seenByConsumer[c][producer]}")
                    }
                    seenByConsumer[c][producer] = popped
                    counts[c][producer]++
                    delivered.addAndFetch(1)
                }
                handle.release()
            })
        }
        threads.forEach { it.join() }
        assertTrue(failures.isEmpty(), failures.take(5).joinToString("; "))
        for (p in 0 until producers) {
            assertEquals(perProducer, counts.sumOf { it[p] }, "every element of producer $p arrived exactly once")
        }
    }
}
