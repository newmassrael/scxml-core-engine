// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.timer_diag_scheduler

import com.sce.forge.runtime.Timer

/**
 * Handler interface for [TimerDiagScheduler]. The user implements these methods
 * on a state class and passes the instance to the [TimerDiagScheduler] constructor.
 * Missing methods produce a compile-time error, so there is no silent fallback.
 */
interface TimerDiagSchedulerHandler {
    fun fireTesterPresent()
    fun fireHandleTimeout()
    fun fireRetrySecurityAccess()
}

class TimerDiagScheduler(
    private val handler: TimerDiagSchedulerHandler,
    private val testerPresentTimer: Timer,
    private val responseTimeoutTimer: Timer,
    private val retryDelayTimer: Timer
) {

    fun startTesterPresent() {
        testerPresentTimer.startPeriodic(2000L) { handler.fireTesterPresent() }
    }

    fun cancelTesterPresent() {
        testerPresentTimer.cancel()
    }

    fun startResponseTimeout() {
        responseTimeoutTimer.startOneShot(5000L) { handler.fireHandleTimeout() }
    }

    fun cancelResponseTimeout() {
        responseTimeoutTimer.cancel()
    }

    fun startRetryDelay() {
        retryDelayTimer.startOneShot(10000L) { handler.fireRetrySecurityAccess() }
    }

    fun cancelRetryDelay() {
        retryDelayTimer.cancel()
    }
}