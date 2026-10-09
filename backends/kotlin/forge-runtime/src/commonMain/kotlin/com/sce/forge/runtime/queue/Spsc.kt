// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * A bounded queue of exactly [capacity] elements for one producer and one
 * consumer: the `bounded` row for that cardinality (RFC §synth-5-P), a Lamport
 * ring, wait-free on both sides.
 *
 * Lap indices, not counters. Each side's index runs over `0..2N` and the slot
 * it names is the index modulo N. Two laps are what tell a full ring from an
 * empty one without spending a slot: the indices are equal exactly when the
 * ring is empty and N apart exactly when it is full, so the ring holds exactly
 * the capacity declared. An index never exceeds 2N, so nothing wraps however
 * long the queue runs.
 *
 * Ordering. The producer publishes a filled slot by storing its index and the
 * consumer reads that index before it reads the slot, so the write of an
 * element happens-before its pop; the consumer returns a slot the same way in
 * the other direction. Each side keeps the other's index as last read, and
 * reads the shared one again only when the cached one says the ring is full
 * (producer) or empty (consumer).
 *
 * The constructor refuses a [capacity] below one by throwing
 * `IllegalArgumentException`. The lap indices run over twice the capacity in a
 * `Long`, which an `Int` capacity can never overflow, so no other size is
 * refused.
 */
public class Spsc<T : Any>(public val capacity: Int) {
    /** The lap index of the next slot the consumer pops. Written only by the consumer. */
    private val head = AtomicLong(0)

    /** The lap index of the next slot the producer fills. Written only by the producer. */
    private val tail = AtomicLong(0)

    private val producerClaimed = AtomicBoolean(false)
    private val consumerClaimed = AtomicBoolean(false)

    private val slotCount: Long
    private val laps: Long
    private val slots: Array<Any?>

    init {
        require(capacity >= 1) { "a queue's capacity must be at least one element" }
        slotCount = capacity.toLong()
        // The size of the lap-index space, twice the capacity.
        laps = 2 * slotCount
        slots = arrayOfNulls(capacity)
    }

    /** The lap index after [index]. */
    private fun next(index: Long): Long = if (index + 1 == laps) 0 else index + 1

    /** The position in the array that a lap index names. */
    private fun slot(index: Long): Int = (if (index >= slotCount) index - slotCount else index).toInt()

    /** How many elements lie between a consumer index and a producer index. */
    private fun occupancy(head: Long, tail: Long): Long = if (tail >= head) tail - head else tail + laps - head

    /**
     * The producing side, or `null` while another is held: a ring shared by two
     * producers is not this algorithm. Release the handle to give the place
     * back.
     */
    public fun producer(): Producer<T>? {
        if (producerClaimed.exchange(true)) {
            return null
        }
        return Producer(this, tail.load(), head.load())
    }

    /**
     * The consuming side, or `null` while another is held. Release the handle
     * to give the place back.
     */
    public fun consumer(): Consumer<T>? {
        if (consumerClaimed.exchange(true)) {
            return null
        }
        return Consumer(this, head.load(), tail.load())
    }

    /** The producing side of an [Spsc]. There is one at a time, and it is for one thread at a time. */
    public class Producer<T : Any> internal constructor(
        private var queue: Spsc<T>?,
        /** This side's own index; the shared one only publishes it. */
        private var tail: Long,
        /** The consumer's index as this side last read it. */
        private var cachedHead: Long,
    ) {
        /**
         * Takes [value], or reports [PushStatus.Full] when the queue holds its
         * capacity. Wait-free: a bounded number of steps whatever the consumer
         * is doing.
         */
        public fun tryPush(value: T): PushStatus {
            val q = checkNotNull(queue) { "the producer was released" }
            if (q.occupancy(cachedHead, tail) == q.slotCount) {
                // Read the consumer's index again: its read of the slot about
                // to be reused happens-before the write below.
                cachedHead = q.head.load()
                if (q.occupancy(cachedHead, tail) == q.slotCount) {
                    return PushStatus.Full
                }
            }
            // The slot at tail is outside [head, tail), so the consumer does not
            // read it until the store below publishes it.
            q.slots[q.slot(tail)] = value
            tail = q.next(tail)
            // The write of the slot happens-before the pop of any consumer that
            // reads this index.
            q.tail.store(tail)
            return PushStatus.Ok
        }

        /** The capacity of the queue this side fills. */
        public val capacity: Int
            get() = checkNotNull(queue) { "the producer was released" }.capacity

        /** Gives the producer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.producerClaimed?.store(false)
            queue = null
        }
    }

    /** The consuming side of an [Spsc]. There is one at a time, and it is for one thread at a time. */
    public class Consumer<T : Any> internal constructor(
        private var queue: Spsc<T>?,
        private var head: Long,
        /** The producer's index as this side last read it. */
        private var cachedTail: Long,
    ) {
        /**
         * Takes the oldest element, or returns `null` when the queue is empty.
         * Wait-free: a bounded number of steps whatever the producer is doing.
         */
        public fun tryPop(): T? {
            val q = checkNotNull(queue) { "the consumer was released" }
            if (head == cachedTail) {
                // Read the producer's index again: its write of the slot
                // happens-before the read below.
                cachedTail = q.tail.load()
                if (head == cachedTail) {
                    return null
                }
            }
            // The slot at head is inside [head, tail), so it holds a pushed
            // element, and the producer does not reuse it until the store below
            // returns it. The slot is cleared so the queue keeps no reference to
            // an element that has left it.
            val index = q.slot(head)
            @Suppress("UNCHECKED_CAST")
            val value = q.slots[index] as T
            q.slots[index] = null
            head = q.next(head)
            // The read of the slot happens-before the producer's reuse of it, by
            // any producer that reads this index.
            q.head.store(head)
            return value
        }

        /** The capacity of the queue this side drains. */
        public val capacity: Int
            get() = checkNotNull(queue) { "the consumer was released" }.capacity

        /** Gives the consumer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.consumerClaimed?.store(false)
            queue = null
        }
    }
}
