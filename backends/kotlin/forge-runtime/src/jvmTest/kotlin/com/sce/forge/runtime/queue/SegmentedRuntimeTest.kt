// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

// The properties of the Kotlin `segmented` runtimes that the contract scenarios
// do not reach (SCE Protocol-Synthesis RFC §synth-5-P): the allocator held to the
// progress the document declared, the places of each side, the segments given
// back to the allocator's account, a ring that stays closed, the slot a pop
// clears, and that real threads lose nothing and reorder nothing between them.

package com.sce.forge.runtime.queue

import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SegmentedRuntimeTest {
    @Test
    fun aRingStaysClosedOnceAPushFoundItWithoutAFreeSlot() {
        val ring = Scq<Long>(2, 2)
        assertTrue(ring.pushOrClose(1))
        assertTrue(ring.pushOrClose(2))
        assertFalse(ring.pushOrClose(3), "no free slot closes the ring")
        assertEquals(1L, ring.popOrNull())
        assertFalse(ring.pushOrClose(4), "a closed ring takes nothing more, though a slot is free again")
        assertEquals(2L, ring.popDrained())
        assertNull(ring.popDrained(), "a closed ring that has been drained will never hold an element")
        assertFalse(ring.pushOrClose(5))
        assertNull(ring.popDrained(), "and a refused push left nothing in it")
    }

    @Test
    fun aSegmentedQueueRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared() {
        val weak = BudgetAllocator(progress = Progress.LockFree)
        for (required in listOf(Progress.Blocking, Progress.LockFree)) {
            assertNotNull(LinkedLamport.create<Long>(2, weak, required))
            assertNotNull(Lscq.create<Long>(2, 2, weak, required))
        }
        assertFailsWith<IllegalArgumentException> { LinkedLamport.create<Long>(2, weak, Progress.WaitFree) }
        assertFailsWith<IllegalArgumentException> { Lscq.create<Long>(2, 2, weak, Progress.WaitFree) }
        assertEquals(4L, weak.live, "the four queues that were built hold one segment each, and the refused ones asked for none")
    }

    @Test
    fun aSegmentedQueueRefusesAShapeItCannotRunBeforeAskingTheAllocator() {
        val allocator = BudgetAllocator()
        assertFailsWith<IllegalArgumentException> { LinkedLamport.create<Long>(0, allocator, Progress.Blocking) }
        assertFailsWith<IllegalArgumentException> { Lscq.create<Long>(0, 1, allocator, Progress.Blocking) }
        assertFailsWith<IllegalArgumentException> { Lscq.create<Long>(2, 3, allocator, Progress.Blocking) }
        assertFailsWith<IllegalArgumentException> { Lscq.create<Long>(4, 2, allocator, Progress.Blocking) }
        assertEquals(0L, allocator.granted, "a bad shape is not an allocator's refusal")
    }

    @Test
    fun aSegmentedQueueThatCannotHaveItsFirstSegmentIsNotBuilt() {
        val none = BudgetAllocator(limit = 0)
        assertNull(LinkedLamport.create<Long>(2, none, Progress.Blocking))
        assertNull(Lscq.create<Long>(2, 2, none, Progress.Blocking))
    }

    @Test
    fun aLinkedQueueGivesEachSegmentBackAsTheConsumerLeavesIt() {
        val allocator = BudgetAllocator()
        val queue = assertNotNull(LinkedLamport.create<Long>(3, allocator, Progress.LockFree))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        repeat(4 * 3 + 1) { assertEquals(PushStatus.Ok, producer.tryPush(it.toLong())) }
        assertEquals(5L, allocator.live, "thirteen elements of three are in five segments")
        repeat(4 * 3 + 1) { assertEquals(it.toLong(), consumer.tryPop()) }
        assertNull(consumer.tryPop())
        assertEquals(1L, allocator.live, "the consumer gave back every segment it left")
    }

    @Test
    fun aLinkedQueueHoldsWhatTheAllocatorAllowsAndIsRoomAgainOnceASegmentIsGivenBack() {
        val queue = assertNotNull(LinkedLamport.create<Long>(2, BudgetAllocator(limit = 2), Progress.LockFree))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        repeat(4) { assertEquals(PushStatus.Ok, producer.tryPush(it.toLong())) }
        assertEquals(PushStatus.OutOfMemory, producer.tryPush(4))
        assertEquals(PushStatus.OutOfMemory, producer.tryPush(4), "a refusal changes nothing, so the next push is refused the same way")
        assertEquals(0L, consumer.tryPop())
        assertEquals(1L, consumer.tryPop())
        assertEquals(2L, consumer.tryPop(), "the consumer left the first segment and gave it back")
        assertEquals(PushStatus.Ok, producer.tryPush(4), "the segment given back is room again")
        assertEquals(3L, consumer.tryPop())
        assertEquals(4L, consumer.tryPop())
        assertNull(consumer.tryPop())
    }

    @Test
    fun aLinkedQueueHasOnePlaceASide() {
        val queue = assertNotNull(LinkedLamport.create<Long>(2, BudgetAllocator(), Progress.Blocking))
        val producer = assertNotNull(queue.producer())
        assertNull(queue.producer(), "a second producer is not this algorithm")
        val consumer = assertNotNull(queue.consumer())
        assertNull(queue.consumer(), "a second consumer is not this algorithm")
        producer.release()
        consumer.release()
        assertNotNull(queue.producer(), "a released place is free again")
        assertNotNull(queue.consumer(), "a released place is free again")
        assertFailsWith<IllegalStateException> { producer.tryPush(1) }
        assertFailsWith<IllegalStateException> { consumer.tryPop() }
    }

    @Test
    fun aLinkedQueueClearsAPoppedSlot() {
        val queue = assertNotNull(LinkedLamport.create<Payload>(2, BudgetAllocator(), Progress.Blocking))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val weak = pushedPayload(7) { producer.tryPush(it) }
        assertEquals(7L, consumer.tryPop()?.value)
        assertTrue(collected(weak), "the queue kept a reference to an element that has left it")
    }

    @Test
    fun aLinkedQueueLosesNothingBetweenThreads() {
        val allocator = BudgetAllocator()
        val queue = assertNotNull(LinkedLamport.create<Long>(4, allocator, Progress.LockFree))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val total = 50_000L
        val failures = ConcurrentLinkedQueue<String>()
        val deadline = System.nanoTime() + DEADLINE_NANOS
        val producing = thread {
            for (value in 0 until total) {
                if (producer.tryPush(value) != PushStatus.Ok) {
                    failures.add("an allocator with no limit refused a segment")
                    return@thread
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
        assertEquals(1L, allocator.live, "every segment but the newest went back")
    }

    @Test
    fun anLscqTakesExactlyASegmentIntoARingAndOnlyTheNewestSegmentSurvivesADrain() {
        for (segment in listOf(1, 2, 3, 5)) {
            val allocator = BudgetAllocator()
            val queue = assertNotNull(Lscq.create<Long>(segment, ceilPow2(segment), allocator, Progress.LockFree))
            val producer = assertNotNull(queue.producer())
            val consumer = assertNotNull(queue.consumer())
            var next = 0L
            var expected = 0L
            repeat(100) {
                repeat(4 * segment + 1) {
                    assertEquals(PushStatus.Ok, producer.tryPush(next))
                    next++
                }
                while (true) {
                    val popped = consumer.tryPop() ?: break
                    assertEquals(expected, popped)
                    expected++
                }
                assertEquals(1L, allocator.live, "segment $segment: only the newest segment is still the queue's")
            }
            assertEquals(next, expected)
            // The first segment, then four more in every round: the segment a
            // round starts in takes a segment's worth and the rest need four.
            assertEquals(401L, allocator.granted, "segment $segment: every round needed four more segments")
        }
    }

    @Test
    fun anLscqRefusedASegmentLeavesTheQueueAsItWasAndLosesNothingPushed() {
        val allocator = BudgetAllocator(limit = 2)
        val queue = assertNotNull(Lscq.create<Long>(2, 2, allocator, Progress.LockFree))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        repeat(4) { assertEquals(PushStatus.Ok, producer.tryPush(it.toLong())) }
        repeat(3) { assertEquals(PushStatus.OutOfMemory, producer.tryPush(99), "refused again, and each time the same") }
        assertEquals(0L, consumer.tryPop())
        assertEquals(1L, consumer.tryPop())
        assertEquals(2L, consumer.tryPop())
        assertEquals(PushStatus.Ok, producer.tryPush(4), "the segment the consumer left is given back: room again")
        assertEquals(3L, consumer.tryPop())
        assertEquals(4L, consumer.tryPop())
        assertNull(consumer.tryPop())
    }

    @Test
    fun anLscqSideHandsOutNoMoreHandlesThanItsRingHasSlots() {
        val queue = assertNotNull(Lscq.create<Long>(2, 4, BudgetAllocator(), Progress.LockFree))
        val producers = List(4) { assertNotNull(queue.producer()) }
        assertNull(queue.producer(), "a ring cannot be shared by more enqueuers than it has slots")
        assertNull(producers[0].tryClone())
        producers[0].release()
        assertNotNull(producers[1].tryClone(), "a released place is free again").release()
        val consumers = List(4) { assertNotNull(queue.consumer()) }
        assertNull(queue.consumer())
        assertNull(consumers[0].tryClone())
    }

    @Test
    fun anLscqClearsAPoppedSlot() {
        val queue = assertNotNull(Lscq.create<Payload>(2, 2, BudgetAllocator(), Progress.LockFree))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val weak = pushedPayload(9) { producer.tryPush(it) }
        assertEquals(9L, consumer.tryPop()?.value)
        assertTrue(collected(weak), "the queue kept a reference to an element that has left it")
    }

    @Test
    fun anLscqLosesNothingAndReordersNothingBetweenThreads() {
        // The allocator allows only a few segments at once, so producers meet
        // refusals and wait for the consumers to give segments back: the
        // refusal and the hand-back run against each other as well.
        val allocator = BudgetAllocator(limit = 6)
        val producers = 3
        val consumers = 3
        val perProducer = 20_000L
        val queue = assertNotNull(Lscq.create<Long>(4, 4, allocator, Progress.LockFree))
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
        assertTrue(allocator.live in 1L..6L, "the allocator's ceiling held and every other segment went back: ${allocator.live}")
    }
}
