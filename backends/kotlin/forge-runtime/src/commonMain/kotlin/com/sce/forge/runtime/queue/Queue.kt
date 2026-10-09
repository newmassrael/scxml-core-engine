// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

/**
 * The runtime half of `sce:kind="queue"` (SCE Protocol-Synthesis RFC
 * §synth-5-P) for Kotlin: queues that hand elements from one thread to
 * another.
 *
 * The document states a contract and names no algorithm; each algorithm the
 * kind's selection table names lives here once. The contract every one of them
 * keeps: a linearizable FIFO in which the push of an element happens-before the
 * pop that returns it, never holding more than the capacity declared and
 * holding exactly that when nothing is running. The SCQ row may refuse a push
 * while another participant's operation holds a slot (at most one per
 * participant); the Lamport row never does.
 *
 * - [Spsc] is the `bounded` row for one producer and one consumer: a Lamport
 *   ring, wait-free on both sides.
 * - [Scq] is the `bounded` row for any other cardinality: Nikolaev's SCQ data
 *   queue (DISC 2019), lock-free on both sides, in the form of the authors'
 *   single-width-CAS reference implementation (lfring_cas1.h, dual 2-clause
 *   BSD / MIT). SCQ has no hardware fetch-or on the JVM, so OR is a
 *   compare-and-swap loop, which fails only because another participant's
 *   operation succeeded and so stays lock-free.
 *
 * The same two algorithms as the Rust, C++ and Go runtimes. The atomics are
 * `kotlin.concurrent.atomics`, which on the JVM are `java.util.concurrent`'s:
 * every operation is sequentially consistent, which is stronger than either
 * algorithm requires and still correct (RFC §synth-5-P, Backends). The forge
 * runtime's only target is `jvm()`, where a `Long` is 64 bits and the
 * architecture is no concern of the queue's.
 *
 * What differs from the languages with destructors and constant generics:
 *
 * - The capacity is an argument of the constructor. The storage is one array,
 *   allocated by the constructor and never again: no operation allocates.
 * - A popped element's slot is cleared, so a queue holds no reference to an
 *   element that has left it and the collector may reclaim it.
 * - A handle holds one place of its side until `release()` is called. There is
 *   no destructor to give the place back, so the caller does, usually in a
 *   `finally`.
 *
 * A refused push needs no hand-back: `tryPush` takes its element by reference,
 * so the caller still holds it.
 */
package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * Whether a push took its element. [OutOfMemory] is the `segmented` storage
 * mode's, reported when its injected allocator refuses a segment; a `bounded`
 * queue only ever reports [Full].
 */
public enum class PushStatus {
    /** The element is in the queue. */
    Ok,

    /**
     * The queue held its capacity, or (SCQ) other participants' operations held
     * the slots it was short of. The element was not taken.
     */
    Full,

    /** A segmented queue's allocator refused a segment. */
    OutOfMemory,
}

/**
 * Takes one of [limit] places of a side, if one is free. A compare-and-swap
 * loop and not an add, so a refused request leaves the count as it was and no
 * concurrent request can be refused on account of it.
 */
internal fun takePlace(alive: AtomicLong, limit: Long): Boolean {
    while (true) {
        val seen = alive.load()
        if (seen >= limit) {
            return false
        }
        if (alive.compareAndSet(seen, seen + 1)) {
            return true
        }
    }
}
