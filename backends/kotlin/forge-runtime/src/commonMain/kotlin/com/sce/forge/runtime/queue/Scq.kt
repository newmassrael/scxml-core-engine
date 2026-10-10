// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * How many operations a participant may be delayed between reading an entry
 * and acting on it before it could be misled: 2^(w-2) for a word of w bits,
 * independent of the capacity. An entry's cycle is compared with a ticket's
 * modulo 2^c, where c = w - 1 - log2(2R), so a delay is safe until the entry's
 * cycle has moved by half that range, 2^(c-1); each cycle takes 2R operations;
 * the product is 2^(w-2). At 10^9 operations a second 2^62 is 146 years (RFC
 * §synth-5-P, Counter width).
 */
public const val WRAP_BOUND_OPS: Long = 1L shl 62

/** The largest ring that leaves an entry enough bits for its cycle. */
internal const val MAX_RING_SLOTS: Int = 1 shl 30

/**
 * How many entries one 64-byte cache line holds, as a power of two.
 * Consecutive tickets are spread over lines by rotating the ticket's bits by
 * this much; a ring smaller than a line does not rotate.
 */
private const val LINE_SHIFT: Int = 3

/**
 * How often a dequeue that found an entry still empty looks again before it
 * makes the entry unusable for the enqueue that owns it. Bounded, so the
 * operation stays lock-free.
 *
 * It is patience and not part of the correctness argument: any count, none
 * included, only changes how soon the entry is given up. A model checker cannot
 * explore a long spin (the Rust runtime's loom build looks once), so a ring takes
 * its count as a parameter and the checker's tests pass a small one; every other
 * ring uses this.
 */
internal const val SPINS: Int = 10_000

/**
 * Whether [x] is before [y] in the order that wraps: the signed difference.
 * Tickets and cycles are compared this way throughout.
 */
private fun before(x: Long, y: Long): Boolean = x - y < 0

/** Whether [x] is [y] or before it, in the same order. */
private fun atOrBefore(x: Long, y: Long): Boolean = x - y <= 0

/** The value a dequeue reports for "nothing": an index is never negative. */
private const val NONE: Long = -1

/**
 * The tail's top bit, set once a ring is closed: an enqueue that takes a ticket
 * from a closed tail gets the bit back and puts nothing on the ring. This is the
 * finalize bit of Nikolaev's LSCQ. The tickets are the bits below it, and a
 * 64-bit counter never reaches it ([WRAP_BOUND_OPS]).
 */
private const val FIN: Long = Long.MIN_VALUE

/**
 * One SCQ ring of R slots (R a power of two) and 2R entries.
 *
 * An entry is cycle | IsSafe | index: the index in the low log2(2R) bits, the
 * IsSafe bit above it, the cycle in the rest. An entry whose index bits are all
 * ones holds nothing. The words are `Long`s used as unsigned: nothing in the
 * algorithm orders them but [before] and [atOrBefore], which are the signed
 * difference, and every shift that must not extend a sign is `ushr`.
 */
private class Ring(slots: Long, filled: Long, private val spins: Int) {
    /** The next ticket a dequeue takes. */
    val head = AtomicLong(0)

    /**
     * How many failed dequeues remain before the ring may be called empty;
     * negative means it is empty.
     */
    val threshold = AtomicLong(0)

    /** The next ticket an enqueue takes. */
    val tail = AtomicLong(0)

    /** The number of entries, 2R; also the value of the IsSafe bit. */
    val entryCount: Long = 2 * slots

    /** The bits an index occupies. */
    val indexMask: Long = entryCount - 1

    /** The index bits and the IsSafe bit: everything below the cycle. */
    val lowMask: Long = 2 * entryCount - 1

    /** log2(2R). */
    val entryOrder: Int = entryCount.countTrailingZeroBits()

    /** 3R - 1: that many failed dequeues are enough to show that an empty-looking ring is empty. */
    val thresholdAfterEnqueue: Long = 3 * slots - 1

    val entries: Array<AtomicLong> = Array(entryCount.toInt()) { position -> AtomicLong(initial(position.toLong(), filled)) }

    init {
        threshold.store(if (filled == 0L) -1L else thresholdAfterEnqueue)
        tail.store(filled)
    }

    /**
     * The entry a ticket names: the ticket rotated so that consecutive tickets
     * land on different cache lines.
     */
    fun mapTicket(ticket: Long): Int =
        (if (entryOrder >= LINE_SHIFT) {
            ((ticket and indexMask) ushr (entryOrder - LINE_SHIFT)) or ((ticket shl LINE_SHIFT) and indexMask)
        } else {
            ticket and indexMask
        }).toInt()

    /** The ticket [mapTicket] sends to this entry. */
    private fun unmap(position: Long): Long =
        if (entryOrder >= LINE_SHIFT) {
            ((position and indexMask) ushr LINE_SHIFT) or ((position shl (entryOrder - LINE_SHIFT)) and indexMask)
        } else {
            position and indexMask
        }

    /**
     * What the entry at [position] holds in a ring built with [filled] indices:
     * the first filled tickets' entries hold index ticket in cycle 0, safe; the
     * rest hold nothing, in the cycle before it.
     */
    private fun initial(position: Long, filled: Long): Long {
        val ticket = unmap(position)
        return if (ticket < filled) entryCount + ticket else -1L
    }

    /**
     * Closes the ring: an enqueue that takes its ticket after this one's
     * fetch-or puts nothing on it. An enqueue that already holds a ticket may
     * still complete; the dequeues that follow either find its entry or make it
     * unusable, which sends that enqueue to the next ticket and so to a refusal.
     */
    fun close() {
        fetchOr(tail, FIN)
    }

    /**
     * Puts [index] on the ring and returns true, or returns false, putting
     * nothing, when the ring is closed. An open ring never refuses: at most R
     * indices circulate through it, and an enqueue that finds its entry unusable
     * takes the next ticket.
     */
    fun enqueue(index: Long): Boolean {
        val stored = index xor indexMask
        while (true) {
            // Looking first keeps a closed ring's tail from counting attempts
            // that cannot succeed; the ticket's own bit is what decides.
            if (tail.load() and FIN != 0L) {
                return false
            }
            val ticket = tail.fetchAndAdd(1)
            if (ticket and FIN != 0L) {
                return false
            }
            val ticketCycle = (ticket shl 1) or lowMask
            val slot = entries[mapTicket(ticket)]
            var entry = slot.load()
            // The entry is ours to fill when it is empty, from an earlier cycle,
            // and either safe or not yet passed by the head.
            while (true) {
                val entryCycle = entry or lowMask
                val usable = before(entryCycle, ticketCycle) &&
                    (entry == entryCycle ||
                        (entry == (entryCycle xor entryCount) && atOrBefore(head.load(), ticket)))
                if (!usable) {
                    break
                }
                if (slot.compareAndSet(entry, ticketCycle xor stored)) {
                    if (threshold.load() != thresholdAfterEnqueue) {
                        threshold.store(thresholdAfterEnqueue)
                    }
                    return true
                }
                // The entry changed; judge what it is now.
                entry = slot.load()
            }
        }
    }

    /**
     * Sets the bits of [mask] in the entry. A compare-and-swap loop: the JVM
     * has no fetch-or, and a loop that fails only because another participant
     * succeeded stays lock-free.
     */
    private fun fetchOr(slot: AtomicLong, mask: Long) {
        while (true) {
            val old = slot.load()
            if (slot.compareAndSet(old, old or mask)) {
                return
            }
        }
    }

    /**
     * Moves tail back to head after dequeues overshot an empty ring, so that
     * the next enqueue does not start a cycle ahead of what is read.
     * [observedTail] is the word as read, closing bit included, and the word
     * written keeps the bit: a ring that was closed stays closed.
     */
    private fun catchUp(observedTail: Long, observedHead: Long) {
        var tailSeen = observedTail
        var headSeen = observedHead
        while (!tail.compareAndSet(tailSeen, headSeen or (tailSeen and FIN))) {
            headSeen = head.load()
            tailSeen = tail.load()
            if (!before(tailSeen and FIN.inv(), headSeen)) {
                break
            }
        }
    }

    /**
     * Reads the entry a dequeue's ticket names until it is settled: the index
     * when the entry was this ticket's (the entry is left empty in the same
     * cycle), [NONE] when the ticket found nothing and the caller goes on to
     * the empty check.
     */
    private fun look(slot: AtomicLong, ticketCycle: Long): Long {
        var looked = 0
        while (true) {
            var entry = slot.load()
            while (true) {
                val entryCycle = entry or lowMask
                if (entryCycle == ticketCycle) {
                    // The entry is this ticket's: take its index and leave the
                    // entry empty in the same cycle.
                    fetchOr(slot, indexMask)
                    return entry and indexMask
                }
                val replacement: Long
                if ((entry or entryCount) != entryCycle) {
                    // It holds an index of another cycle: mark it unsafe, so no
                    // enqueue of this cycle's ticket fills the entry behind the
                    // head. Already unsafe: nothing to do.
                    replacement = entry and entryCount.inv()
                    if (entry == replacement) {
                        return NONE
                    }
                } else {
                    // It is empty. An enqueue holding this cycle's ticket may be
                    // about to fill it; look a few times, then move the entry to
                    // this cycle so that enqueue fails.
                    looked++
                    if (looked <= spins) {
                        break // look again from a fresh read
                    }
                    replacement = ticketCycle xor (entry.inv() and entryCount)
                }
                if (!before(entryCycle, ticketCycle)) {
                    return NONE
                }
                if (slot.compareAndSet(entry, replacement)) {
                    return NONE
                }
                entry = slot.load()
            }
        }
    }

    /**
     * Takes the oldest index off the ring, or returns [NONE] when it is empty.
     * The empty answer may come from the threshold alone, which does not look at
     * a ticket an enqueue holds and has not yet filled. With [byThreshold] false
     * the threshold neither ends the dequeue early nor is spent by it: the head
     * walks to the tail, which a closed ring no longer moves, so an enqueue still
     * holding a ticket either finds its entry taken from it (and takes the next
     * ticket, which is refused) or has already filled it, and then the walk
     * returns its index.
     */
    fun dequeue(byThreshold: Boolean = true): Long {
        if (byThreshold && threshold.load() < 0) {
            return NONE
        }
        while (true) {
            val ticket = head.fetchAndAdd(1)
            val ticketCycle = (ticket shl 1) or lowMask
            val index = look(entries[mapTicket(ticket)], ticketCycle)
            if (index != NONE) {
                return index
            }

            val tailSeen = tail.load()
            if (atOrBefore(tailSeen and FIN.inv(), ticket + 1)) {
                catchUp(tailSeen, ticket + 1)
                if (byThreshold) {
                    threshold.addAndFetch(-1)
                }
                return NONE
            }
            // addAndFetch returns the new value; the value before was positive
            // exactly when the new one is not below zero.
            if (byThreshold && threshold.addAndFetch(-1) < 0) {
                return NONE
            }
        }
    }
}

/**
 * A bounded queue of at most [capacity] elements for any number of producers
 * and any number of consumers, lock-free. Exactly [capacity] fit when nothing
 * else is running.
 *
 * The elements live in an array of capacity slots. Two rings of indices make it
 * a data queue: the free ring holds the indices of the slots nobody is using
 * and starts with all of them; the allocated ring holds the indices of the
 * slots that hold an element, in the order they were filled, and starts empty.
 * A push takes an index from the free ring (none: the queue is full), writes
 * its element into that slot and puts the index on the allocated ring; a pop
 * does the reverse. The rings carry only indices.
 *
 * The ring has [ringSlots] slots, a power of two at least [capacity], and at
 * least the most producers or consumers that will work the queue at once. The
 * algorithm's empty test is justified for at most ringSlots enqueuers and at
 * most ringSlots dequeuers working one ring at once (Nikolaev 2019, section
 * 5.1); beyond that a completed push can leave the threshold at -1 with its
 * element in the ring and every pop reports the queue empty until another push
 * completes. So a queue hands out at most ringSlots producer handles and at
 * most ringSlots consumer handles at a time.
 *
 * A push may be refused while others hold slots: a slot's index goes back to
 * the free ring only after the pop that took the element has read it, and a
 * push holds the index it took until it has published its element. Waiting for
 * the operation that holds the slot would not be lock-free, since that
 * participant may be stopped. The shortfall is at most one slot per other
 * participant.
 *
 * The constructor refuses a capacity below one, a [ringSlots] that is not a
 * power of two, is below [capacity], or is more than a ring can number, by
 * throwing `IllegalArgumentException`.
 */
public class Scq<T : Any> internal constructor(public val capacity: Int, public val ringSlots: Int, spins: Int) {
    /** A queue whose rings spin [SPINS] times on an entry that is still empty. */
    public constructor(capacity: Int, ringSlots: Int) : this(capacity, ringSlots, SPINS)

    /** The indices of the slots that hold an element, oldest first. */
    private val allocated: Ring

    /** The indices of the slots nobody is using. */
    private val free: Ring

    private val producers = AtomicLong(0)
    private val consumers = AtomicLong(0)

    private val slots: Array<Any?>

    init {
        checkShape(capacity, ringSlots)
        allocated = Ring(ringSlots.toLong(), 0, spins)
        free = Ring(ringSlots.toLong(), capacity.toLong(), spins)
        slots = arrayOfNulls(capacity)
    }

    internal companion object {
        /**
         * Refuses a shape no ring can have, by throwing `IllegalArgumentException`.
         * A list of rings checks it before it asks its allocator for a segment, so
         * a bad shape is not reported as an allocator's refusal.
         */
        fun checkShape(capacity: Int, ringSlots: Int) {
            require(capacity >= 1) { "a queue's capacity must be at least one element" }
            require(ringSlots >= 1 && ringSlots and (ringSlots - 1) == 0) { "the ring size must be a power of two" }
            require(ringSlots >= capacity) { "the ring must have at least as many slots as the capacity" }
            require(ringSlots <= MAX_RING_SLOTS) { "a ring this large leaves its entries too few bits for the cycle" }
        }
    }

    /**
     * Takes [value] and returns true, or returns false and takes nothing: when
     * the queue has no free slot it is closed, and when it is closed already
     * nothing more is ever pushed into it, which is what lets a list of rings put
     * every later element behind this ring's. The queue's places are not
     * consulted: the list of rings that owns this ring keeps its own. Lock-free.
     */
    internal fun pushOrClose(value: T): Boolean {
        val index = free.dequeue()
        if (index == NONE) {
            allocated.close()
            return false
        }
        slots[index.toInt()] = value
        if (allocated.enqueue(index)) {
            return true
        }
        // The ring was closed under the push: the index is still ours, and goes
        // back to the free ring, which is never closed.
        slots[index.toInt()] = null
        free.enqueue(index)
        return false
    }

    /** Takes the oldest element, or returns `null` when the ring is empty. Lock-free. */
    internal fun popOrNull(): T? = takeFrom(allocated.dequeue())

    /**
     * Takes the oldest element of a closed ring, or returns `null` only when no
     * element can ever arrive in it again. A ring is given up only after this
     * answers `null`: until then an element may still arrive in it. Lock-free.
     */
    internal fun popDrained(): T? = takeFrom(allocated.dequeue(byThreshold = false))

    /**
     * The element of the slot [index] named, which came off the allocated ring,
     * so the slot holds a pushed element and no other participant holds it until
     * the enqueue below hands it on. The slot is cleared so the queue keeps no
     * reference to an element that has left it.
     */
    private fun takeFrom(index: Long): T? {
        if (index == NONE) {
            return null
        }
        @Suppress("UNCHECKED_CAST")
        val value = slots[index.toInt()] as T
        slots[index.toInt()] = null
        free.enqueue(index)
        return value
    }

    /**
     * A producer handle, or `null` when every producer place is taken: the ring
     * cannot be shared by more enqueuers than it has slots. Release the handle
     * to give the place back.
     */
    public fun producer(): Producer<T>? = if (takePlace(producers, ringSlots.toLong())) Producer(this) else null

    /**
     * A consumer handle, or `null` when every consumer place is taken. Release
     * the handle to give the place back.
     */
    public fun consumer(): Consumer<T>? = if (takePlace(consumers, ringSlots.toLong())) Consumer(this) else null

    /** A producing side of an [Scq]. It holds one of the queue's producer places until [release]. */
    public class Producer<T : Any> internal constructor(private var queue: Scq<T>?) {
        /** Another producer of the same queue, or `null` when every place is taken. */
        public fun tryClone(): Producer<T>? = checkNotNull(queue) { "the producer was released" }.producer()

        /**
         * Takes [value], or reports [PushStatus.Full] when the queue holds its
         * capacity, or while other participants' operations hold the slots it
         * is short of. Lock-free: some operation completes in a bounded number
         * of steps whatever the other participants are doing.
         */
        public fun tryPush(value: T): PushStatus {
            val q = checkNotNull(queue) { "the producer was released" }
            val index = q.free.dequeue()
            if (index == NONE) {
                return PushStatus.Full
            }
            // The index came off the free ring, so no other participant holds
            // this slot until the enqueue below hands it on.
            q.slots[index.toInt()] = value
            q.allocated.enqueue(index)
            return PushStatus.Ok
        }

        /** The capacity of the queue this side fills. */
        public val capacity: Int
            get() = checkNotNull(queue) { "the producer was released" }.capacity

        /** Gives the producer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.producers?.addAndFetch(-1)
            queue = null
        }
    }

    /** A consuming side of an [Scq]. It holds one of the queue's consumer places until [release]. */
    public class Consumer<T : Any> internal constructor(private var queue: Scq<T>?) {
        /** Another consumer of the same queue, or `null` when every place is taken. */
        public fun tryClone(): Consumer<T>? = checkNotNull(queue) { "the consumer was released" }.consumer()

        /** Takes the oldest element, or returns `null` when the queue is empty. Lock-free. */
        public fun tryPop(): T? = checkNotNull(queue) { "the consumer was released" }.popOrNull()

        /** The capacity of the queue this side drains. */
        public val capacity: Int
            get() = checkNotNull(queue) { "the consumer was released" }.capacity

        /** Gives the consumer place back. The handle is not usable afterwards. */
        public fun release() {
            queue?.consumers?.addAndFetch(-1)
            queue = null
        }
    }
}
