// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.queue

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * The allocator the segmented queues' tests inject (SCE Protocol-Synthesis RFC
 * §synth-5-P): it gives at most [limit] segments at once, counting the ones it
 * has out so a run can say that the queue gave every segment back. [limit] is
 * the memory ceiling a bounded allocator stands for. The count is an atomic:
 * the queue calls the allocator from several threads, and a budget taken with a
 * compare-and-swap loop refuses no request on account of another's.
 *
 * Its [progress] is what a test says the allocator gives, so a test can inject
 * an allocator that gives less than a queue's document declared for it.
 */
internal class BudgetAllocator(
    private val limit: Long = Long.MAX_VALUE,
    override val progress: Progress = Progress.LockFree,
) : SegmentAllocator {
    private val out = AtomicLong(0)
    private val given = AtomicLong(0)

    /** The segments the queue holds or has been given and not handed back. */
    val live: Long
        get() = out.load()

    /** How many segments were granted in all. */
    val granted: Long
        get() = given.load()

    override fun <S : Any> allocate(make: () -> S): S? {
        while (true) {
            val seen = out.load()
            if (seen >= limit) {
                return null
            }
            if (out.compareAndSet(seen, seen + 1)) {
                break
            }
        }
        given.addAndFetch(1)
        return make()
    }

    override fun deallocate(segment: Any) {
        out.addAndFetch(-1)
    }
}
