// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The one way a Kotlin test waits for a machine: it owns the time.

package com.sce.w3c

import com.sce.runtime.Event
import com.sce.runtime.ManualClock
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine

/**
 * Start [this] on a clock the test owns.
 *
 * The clock is installed before `initialize()` because entering the initial
 * configuration arms every `<send delay>` in it against whatever clock is
 * there at that moment, and the setter refuses a swap afterwards for exactly
 * that reason.
 */
fun <S : State, E : Event> StateMachineEngine<S, E>.initializeOnVirtualTime() {
    clock = ManualClock()
    initialize()
}

/**
 * Drive a machine started with [initializeOnVirtualTime] until [done] holds,
 * moving its clock only to the next moment the machine says it is owed work.
 *
 * Returns whether [done] held when the drive ended. A drive ends without it when
 * the machine owes nothing more, or when the next thing it owes lies beyond
 * [budgetMs] of virtual time; neither depends on how fast the host runs this
 * thread, so the same calls reach the same configuration on an idle machine
 * and on one running a thousand tests at once.
 *
 * Why not a wall-clock deadline polled with `Thread.sleep(10); tick()`: that
 * loop makes the verdict a race between how soon this thread is scheduled and
 * the document's own timers. A test that finished in 40ms alone failed under a
 * parallel run of the whole module (`ATargetExpressionIsRoutedAsItsLiteralIs`
 * passed three runs in three alone and failed under the full one), and
 * lengthening the deadline only moves the mark: the document's timers are armed
 * in the same real seconds. Virtual time removes the race, so [budgetMs] bounds
 * how long the DOCUMENT may take rather than how long the host may be late.
 *
 * Tick before advancing. Not every reason a machine is not finished is on the
 * clock: an invoked child with nothing scheduled still needs its parent's tick
 * to run its queues and to be noticed finishing, and it reports no deadline for
 * that.
 */
fun <S : State, E : Event> StateMachineEngine<S, E>.driveOnVirtualTime(
    budgetMs: Long = VirtualDrive.DEFAULT_BUDGET_MS,
    done: () -> Boolean = { isInFinalState },
): Boolean {
    var remaining = budgetMs
    var steps = 0
    while (!done()) {
        tick()
        if (done()) {
            break
        }
        // `null` is "nothing is owed", by this machine or any child it ticks.
        // The tick above is already done, so no further time can change that.
        val due = timeUntilNextScheduledMs() ?: break
        if (due > remaining) {
            break
        }
        advanceTimeMs(due)
        remaining -= due
        if (++steps > VirtualDrive.MAX_STEPS) {
            throw AssertionError(
                "the machine asked to be ticked ${VirtualDrive.MAX_STEPS} times without " +
                    "reaching what the test waits for or consuming its ${budgetMs}ms budget; " +
                    "a send that re-arms itself with no delay is the usual cause"
            )
        }
    }
    return done()
}

object VirtualDrive {
    /**
     * The virtual time a document may take, which is the budget the Rust and Go
     * drivers already give the same documents.
     */
    const val DEFAULT_BUDGET_MS: Long = 3000L

    /**
     * How many times a drive advances virtual time before it calls the machine
     * stuck. A budget in STEPS as well as in milliseconds, because virtual time
     * does not bound the loop on its own: a `<send>` that re-arms itself with
     * `delay="0s"` is always due now, so the loop would tick forever without the
     * clock moving. Finite, so a runaway fails with a message instead of being
     * killed by a JUnit timeout, which would put the wall clock back in the
     * verdict.
     */
    const val MAX_STEPS: Int = 10_000
}
