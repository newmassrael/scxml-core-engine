// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Runtime — the engine's source of "now"

package com.sce.runtime

import kotlin.time.TimeSource

/**
 * The source of "now" behind every `<send delay>` a [StateMachineEngine] arms
 * and every due judgement its `tick` makes.
 *
 * §scxml-6.2.2 says a delay "indicates how long the processor should wait
 * before dispatching the message", and says nothing about where the processor
 * reads the time from. Leaving that hardwired to the wall answers a question
 * the spec left to the host, and answers it the one way that cannot be
 * reproduced: a host descheduled between two statements of the same `<onentry>`
 * gets two different readings for one instant, and the deadlines it computes
 * from them can order the sends differently on every run.
 *
 * So the reading is a seam, not a constant. [MonotonicClock] is the default and
 * is what a production host wants; [ManualClock] hands the clock to the host
 * outright, which is what a simulation, a replay, a discrete-event scheduler
 * and a deterministic test all want. Both are runtime types on the shipped
 * surface — a consumer can install either, or write a third.
 */
interface SceClock {
    /**
     * Milliseconds elapsed since this clock's origin.
     *
     * Must be non-decreasing: the scheduler compares readings taken at
     * different moments and a reading that went backwards would make an entry
     * that was due stop being due.
     */
    fun elapsedMs(): Long
}

/**
 * The default [SceClock] — a monotonic reading of the host's wall clock,
 * measured from the moment this clock was constructed.
 *
 * This is what an engine gets when nothing else is installed, and what a
 * production host running against real time should keep.
 */
class MonotonicClock : SceClock {
    private val origin = TimeSource.Monotonic.markNow()

    override fun elapsedMs(): Long = origin.elapsedNow().inWholeMilliseconds
}

/**
 * A [SceClock] the host moves by hand.
 *
 * Time advances only when [advance] is called, so a machine driven through one
 * of these reaches the same configuration on every run regardless of what else
 * the machine it runs on is doing. That is what makes it the right clock for a
 * simulation, for replaying a recorded trace at a speed of the host's
 * choosing, and for a test that wants a verdict about the engine rather than
 * about the load on the build machine.
 *
 * Install it before [StateMachineEngine.initialize], and drive the machine
 * with [StateMachineEngine.advanceTimeMs] rather than calling [advance]
 * directly — the engine's entry point moves this clock and then runs whatever
 * that made due, which is the whole of the contract.
 *
 * One instance may be shared by several engines; an invoked child inherits its
 * parent's, so parent and child read the same absolute time (§scxml-6.4).
 */
class ManualClock(startMs: Long = 0L) : SceClock {
    private var nowMs: Long = startMs

    init {
        require(startMs >= 0L) { "ManualClock origin must not be negative: $startMs" }
    }

    override fun elapsedMs(): Long = nowMs

    /**
     * Move this clock forward by [ms] milliseconds.
     *
     * Rejects a negative delta rather than accepting it: [SceClock.elapsedMs]
     * is required to be non-decreasing, and a clock that went backwards would
     * un-due an entry the scheduler had already judged ready.
     */
    fun advance(ms: Long) {
        require(ms >= 0L) { "ManualClock.advance requires a non-negative delta: $ms" }
        nowMs += ms
    }
}
