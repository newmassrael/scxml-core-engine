// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * One segment: [slots] filled once, in order. A Lamport ring that is never
 * lapped, so it needs neither the second lap that tells a full ring from an
 * empty one nor a free slot.
 */
internal class LamportSegment<T : Any>(size: Int) {
    /** How many slots, from the first, hold an element the producer published. Written only by the producer. */
    val written = AtomicLong(0)

    /** The segment after this one, or `null` while this is the newest. Stored once, by the producer. */
    val next = AtomicReference<LamportSegment<T>?>(null)

    val slots: Array<Any?> = arrayOfNulls(size)
}

/**
 * The `segmented` row for one producer and one consumer (SCE Protocol-Synthesis
 * RFC §synth-5-P): linked Lamport rings, wait-free on both sides, bounded only by
 * the allocator it is given.
 *
 * The queue is a list of segments of [segment] elements. The producer fills the
 * newest segment and, when it is full, takes a new one from the allocator and
 * links it behind; the consumer reads the oldest segment and, when it has read
 * all of it, follows the link and hands the segment back to the allocator. Each
 * side keeps its own position and shares it with the other only through the two
 * things the other must see: how many slots of a segment are filled, and the link
 * to the next segment.
 *
 * Pop is wait-free. Push is wait-free but for the one step that can take a
 * segment: the allocator. The queue is therefore as strong as the allocator the
 * document declares, which [create] holds to the declared [Progress]. A push the
 * allocator refuses reports [PushStatus.OutOfMemory] and leaves the element with
 * the caller.
 *
 * Ordering. The producer publishes a filled slot by storing the count of filled
 * slots and the consumer reads that count before it reads the slot, so the write
 * of an element happens-before the pop that returns it. The link is stored and
 * read the same way: everything the producer did to a segment before linking it
 * happens-before the consumer's use of the next one. A segment the consumer has
 * left is the collector's; no reclamation scheme is needed.
 */
public class LinkedLamport<T : Any> private constructor(
    /** The elements one segment holds. */
    public val segment: Int,
    private val allocator: SegmentAllocator,
    first: LamportSegment<T>,
) {
    /** The segment the consumer reads. Written only by the consumer. */
    private val head = AtomicReference(first)

    /** How many elements of the head segment the consumer has taken. Written only by the consumer. */
    private val headRead = AtomicLong(0)

    /** The segment the producer fills. Written only by the producer. */
    private val tail = AtomicReference(first)

    private val producerClaimed = AtomicBoolean(false)
    private val consumerClaimed = AtomicBoolean(false)

    public companion object {
        /**
         * An empty queue of segments of [segment] elements over [allocator], or
         * `null` when the allocator will not give the first segment. Throws
         * `IllegalArgumentException` for a segment below one element, and for an
         * allocator whose [SegmentAllocator.progress] is below [required], the
         * progress the document declared for it: a queue that would claim more
         * than its allocator gives does not build.
         */
        public fun <T : Any> create(segment: Int, allocator: SegmentAllocator, required: Progress): LinkedLamport<T>? {
            require(segment >= 1) { "a segment holds at least one element" }
            require(allocator.progress >= required) {
                "the allocator gives ${allocator.progress}, less than the $required the document declared for it"
            }
            val first = allocator.allocate { LamportSegment<T>(segment) } ?: return null
            return LinkedLamport(segment, allocator, first)
        }
    }

    /**
     * The producing side, or `null` while another is held: a list shared by two
     * producers is not this algorithm. Release the handle to give the place back.
     */
    public fun producer(): Producer<T>? = if (producerClaimed.exchange(true)) null else Producer(this)

    /**
     * The consuming side, or `null` while another is held. Release the handle to
     * give the place back.
     */
    public fun consumer(): Consumer<T>? = if (consumerClaimed.exchange(true)) null else Consumer(this)

    /** The producing side of a [LinkedLamport]. There is one at a time, and it is for one thread at a time. */
    public class Producer<T : Any> internal constructor(private var queue: LinkedLamport<T>?) {
        /**
         * Takes [value], or reports [PushStatus.OutOfMemory] when the newest
         * segment is full and the allocator will not give another; the element is
         * the caller's either way. Wait-free when the allocator is.
         */
        public fun tryPush(value: T): PushStatus {
            val q = checkNotNull(queue) { "the producer was released" }
            // The tail and the filled count are this side's: only the producer
            // writes them.
            var tail = q.tail.load()
            var written = tail.written.load().toInt()
            if (written == q.segment) {
                val fresh = q.allocator.allocate { LamportSegment<T>(q.segment) } ?: return PushStatus.OutOfMemory
                // This is the only store of the link, and the producer never
                // touches the segment again after it.
                tail.next.store(fresh)
                q.tail.store(fresh)
                tail = fresh
                written = 0
            }
            // Slot `written` is past every filled slot, so the consumer does not
            // read it until the store below publishes it.
            tail.slots[written] = value
            tail.written.store((written + 1).toLong())
            return PushStatus.Ok
        }

        /** The elements one segment of the queue this side fills holds. */
        public val segment: Int
            get() = checkNotNull(queue) { "the producer was released" }.segment

        /** Gives the producer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.producerClaimed?.store(false)
            queue = null
        }
    }

    /** The consuming side of a [LinkedLamport]. There is one at a time, and it is for one thread at a time. */
    public class Consumer<T : Any> internal constructor(private var queue: LinkedLamport<T>?) {
        /** Takes the oldest element, or returns `null` when the queue is empty. Wait-free. */
        public fun tryPop(): T? {
            val q = checkNotNull(queue) { "the consumer was released" }
            var head = q.head.load()
            var read = q.headRead.load().toInt()
            if (read == q.segment) {
                // Every element of the head segment is taken. Its successor
                // exists once the producer has linked it, and the producer never
                // touches this segment after that, so it is ours to give back.
                val next = head.next.load() ?: return null
                q.allocator.deallocate(head)
                head = next
                read = 0
                q.head.store(head)
                q.headRead.store(0)
            }
            if (read >= head.written.load()) {
                return null
            }
            // Slot `read` is below the published count, so it holds an element,
            // and only this side reads it. It is cleared so the queue keeps no
            // reference to an element that has left it.
            @Suppress("UNCHECKED_CAST")
            val value = head.slots[read] as T
            head.slots[read] = null
            q.headRead.store((read + 1).toLong())
            return value
        }

        /** The elements one segment of the queue this side drains holds. */
        public val segment: Int
            get() = checkNotNull(queue) { "the consumer was released" }.segment

        /** Gives the consumer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.consumerClaimed?.store(false)
            queue = null
        }
    }
}
