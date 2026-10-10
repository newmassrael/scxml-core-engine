// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/** One segment of an [Lscq]: a ring and the link to the next. */
internal class RingSegment<T : Any>(segment: Int, ringSlots: Int, spins: Int) {
    /** The segment after this one, or `null` while this is the newest. Linked once, by a compare-and-swap. */
    val next = AtomicReference<RingSegment<T>?>(null)

    val ring = Scq<T>(segment, ringSlots, spins)
}

/**
 * The `segmented` row for any cardinality other than one producer and one
 * consumer (SCE Protocol-Synthesis RFC §synth-5-P): LSCQ, a list of SCQ rings,
 * lock-free on both sides and bounded only by the allocator it is given.
 *
 * The algorithm is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott list
 * whose nodes are SCQ rings instead of single elements. Producers push into the
 * newest ring (`tail`); consumers pop from the oldest (`head`). A ring that
 * cannot take another element is closed: nothing is ever pushed into it again.
 * The producer that found it closed asks the allocator for a segment, puts its
 * element in it before anyone can see it, and links it behind the closed one with
 * a compare-and-swap on that ring's `next`; a producer that loses that race
 * hands its segment back and pushes into the winner's. A consumer that finds the
 * oldest ring empty and closed moves `head` to its successor.
 *
 * Each ring is the SCQ data queue of [Scq] with the close bit its tail carries.
 * A ring is closed by the first push that finds it without a free slot, and an
 * enqueue that takes its ticket after the close is refused, which is what makes
 * the order across segments exact. A ring is left only after `popDrained`, which
 * walks the head to the closed tail and either takes the element an enqueue
 * still holding a ticket filled or makes its entry unusable; without it a
 * producer that passed a "not closed" check and stalled could push into a
 * segment a consumer had already emptied and left, and the element would be
 * lost.
 *
 * Reclamation is the collector's. A segment can be read by a participant after
 * another has moved `head` past it, and the collector keeps it alive for that
 * participant: no hazard-pointer domain is needed, which is the one thing a
 * collected runtime does not build that the Rust, C++ and C11 ones do. A segment
 * is given back to the allocator's account by the consumer whose compare-and-swap
 * moved `head` past it, and only by that one.
 *
 * Push and pop are lock-free: a retry means another participant moved `head`,
 * `tail` or a ring, and the helping step lets a push finish what a stalled one
 * started. A push is as strong as the allocator it calls, which [create] holds to
 * the declared [Progress]; a refusal reports [PushStatus.OutOfMemory] and leaves
 * the element with the caller.
 *
 * Each segment's ring has [ringSlots] slots, a power of two at least [segment]
 * and at least the most producers or consumers that work the queue at once: its
 * empty test is justified for at most that many enqueuers and that many dequeuers
 * on one ring (Nikolaev 2019, section 5.1). So the queue hands out at most
 * [ringSlots] producer handles and as many consumer handles at a time.
 */
public class Lscq<T : Any> private constructor(
    /** The elements one segment holds. */
    public val segment: Int,
    /** The slots of each segment's index rings. */
    public val ringSlots: Int,
    private val allocator: SegmentAllocator,
    /** How often a ring's dequeue looks again at an entry still empty; see [SPINS]. */
    private val spins: Int,
    first: RingSegment<T>,
) {
    /** The oldest segment. */
    private val head = AtomicReference(first)

    /** The newest segment, or one behind it while a producer has linked a successor and not yet moved this. */
    private val tail = AtomicReference(first)

    private val producers = AtomicLong(0)
    private val consumers = AtomicLong(0)

    public companion object {
        /**
         * An empty queue of segments of [segment] elements over [allocator], or
         * `null` when the allocator will not give the first segment. Throws
         * `IllegalArgumentException` for a [segment] or [ringSlots] no ring can
         * have (see [Scq]), and for an allocator whose
         * [SegmentAllocator.progress] is below [required], the progress the
         * document declared for it.
         */
        public fun <T : Any> create(
            segment: Int,
            ringSlots: Int,
            allocator: SegmentAllocator,
            required: Progress,
        ): Lscq<T>? = create(segment, ringSlots, allocator, required, SPINS)

        /** [create] with the rings' spin count named: what a model checker's tests use. */
        internal fun <T : Any> create(
            segment: Int,
            ringSlots: Int,
            allocator: SegmentAllocator,
            required: Progress,
            spins: Int,
        ): Lscq<T>? {
            Scq.checkShape(segment, ringSlots)
            require(allocator.progress >= required) {
                "the allocator gives ${allocator.progress}, less than the $required the document declared for it"
            }
            val first = allocator.allocate { RingSegment<T>(segment, ringSlots, spins) } ?: return null
            return Lscq(segment, ringSlots, allocator, spins, first)
        }
    }

    /**
     * A producer handle, or `null` when every producer place is taken: a ring
     * cannot be shared by more enqueuers than it has slots. Release the handle to
     * give the place back.
     */
    public fun producer(): Producer<T>? = if (takePlace(producers, ringSlots.toLong())) Producer(this) else null

    /**
     * A consumer handle, or `null` when every consumer place is taken. Release the
     * handle to give the place back.
     */
    public fun consumer(): Consumer<T>? = if (takePlace(consumers, ringSlots.toLong())) Consumer(this) else null

    /** A producing side of an [Lscq]. It holds one of the queue's producer places until [release]. */
    public class Producer<T : Any> internal constructor(private var queue: Lscq<T>?) {
        /** Another producer of the same queue, or `null` when every place is taken. */
        public fun tryClone(): Producer<T>? = checkNotNull(queue) { "the producer was released" }.producer()

        /**
         * Takes [value], or reports [PushStatus.OutOfMemory] when the newest
         * segment is closed and the allocator will not give another; the element
         * is the caller's either way. Lock-free when the allocator is.
         */
        public fun tryPush(value: T): PushStatus {
            val q = checkNotNull(queue) { "the producer was released" }
            while (true) {
                val tail = q.tail.load()
                val next = tail.next.load()
                if (next != null) {
                    // A producer linked a successor and has not moved `tail`:
                    // finish its step.
                    q.tail.compareAndSet(tail, next)
                    continue
                }
                if (tail.ring.pushOrClose(value)) {
                    return PushStatus.Ok
                }
                // The segment is closed. Put the element in a segment of our own
                // before anyone can see it, then try to link it behind this one.
                val fresh = q.allocator.allocate { RingSegment<T>(q.segment, q.ringSlots, q.spins) }
                    ?: return PushStatus.OutOfMemory
                // A new ring is open and has a free slot, so it takes an element.
                check(fresh.ring.pushOrClose(value)) { "a new ring refused its first element" }
                if (tail.next.compareAndSet(null, fresh)) {
                    q.tail.compareAndSet(tail, fresh)
                    return PushStatus.Ok
                }
                // Another producer linked first. The segment nobody saw goes back
                // to the allocator's account, and the element, which this side
                // still holds, is pushed behind theirs.
                q.allocator.deallocate(fresh)
            }
        }

        /** The elements one segment of the queue this side fills holds. */
        public val segment: Int
            get() = checkNotNull(queue) { "the producer was released" }.segment

        /** Gives the producer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.producers?.addAndFetch(-1)
            queue = null
        }
    }

    /** A consuming side of an [Lscq]. It holds one of the queue's consumer places until [release]. */
    public class Consumer<T : Any> internal constructor(private var queue: Lscq<T>?) {
        /** Another consumer of the same queue, or `null` when every place is taken. */
        public fun tryClone(): Consumer<T>? = checkNotNull(queue) { "the consumer was released" }.consumer()

        /** Takes the oldest element, or returns `null` when the queue is empty. Lock-free. */
        public fun tryPop(): T? {
            val q = checkNotNull(queue) { "the consumer was released" }
            while (true) {
                val head = q.head.load()
                head.ring.popOrNull()?.let { return it }
                val next = head.next.load() ?: return null
                // The segment is closed. An enqueue that took a ticket before the
                // close may still be filling its entry, so the empty answer above
                // does not yet show the segment holds nothing and never will.
                head.ring.popDrained()?.let { return it }
                if (q.head.compareAndSet(head, next)) {
                    // This consumer moved `head` past the segment, so it alone
                    // gives it back.
                    q.allocator.deallocate(head)
                }
            }
        }

        /** The elements one segment of the queue this side drains holds. */
        public val segment: Int
            get() = checkNotNull(queue) { "the consumer was released" }.segment

        /** Gives the consumer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.consumers?.addAndFetch(-1)
            queue = null
        }
    }
}
