// SCE-MAP: timer_diag_scheduler:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Shape: watching-zenoh RFC §5.D line 880-886 — single timer per
// doc with event-driven reset / state-exit cancel / fire event.
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.timer_diag_scheduler

import com.sce.forge.runtime.Timer

/**
 * Period configured at compile time from `<sce:period>`.
 * Microseconds; cover MCU microsecond ticks through minute-scale
 * watchdogs in one type.
 */
const val PERIOD_US: Long = 2000000L
const val PERIOD_MS: Long = 2000L
const val RESET_ON_EVENT: String = "diag.heartbeat"
const val CANCEL_ON_STATE_EXIT: String = "diag.idle"

/**
 * Handler interface for [TimerDiagScheduler]. The user implements the
 * fire method on a state class and passes the instance to the
 * [TimerDiagScheduler] constructor.
 */
interface TimerDiagSchedulerHandler {
    fun fireDiagTick()
}

class TimerDiagScheduler(
    private val handler: TimerDiagSchedulerHandler,
    private val timer: Timer
) {

    /** Start the periodic timer at compile-time `PERIOD_MS`. */
    fun start() {
        timer.startPeriodic(PERIOD_MS) { handler.fireDiagTick() }
    }

    /** Cancel the timer. Idempotent per the runtime contract. */
    fun cancel() {
        timer.cancel()
    }

    /** `<sce:reset-on event="diag.heartbeat"/>` consumer hook. */
    fun onResetDiagHeartbeat() {
        cancel()
        start()
    }

    /** `<sce:cancel-on state-exit="diag.idle"/>` consumer hook. */
    fun onCancelDiagIdleExit() {
        cancel()
    }
}
