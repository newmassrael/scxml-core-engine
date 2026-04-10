// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

interface ITimer {
    fun startPeriodic(intervalMs: Long, callback: () -> Unit)
    fun startOneShot(delayMs: Long, callback: () -> Unit)
    fun cancel()
}

class TimerDiagScheduler {
    var testerPresentTimer: ITimer? = null
    var responseTimeoutTimer: ITimer? = null
    var retryDelayTimer: ITimer? = null

    fun startTesterpresent() {
        testerPresentTimer?.startPeriodic(2000L) { onTesterpresent() }
    }

    fun cancelTesterpresent() {
        testerPresentTimer?.cancel()
    }

    fun startResponsetimeout() {
        responseTimeoutTimer?.startOneShot(5000L) { onHandletimeout() }
    }

    fun cancelResponsetimeout() {
        responseTimeoutTimer?.cancel()
    }

    fun startRetrydelay() {
        retryDelayTimer?.startOneShot(10000L) { onRetrysecurityaccess() }
    }

    fun cancelRetrydelay() {
        retryDelayTimer?.cancel()
    }

    private fun onTesterpresent() { /* platform callback */ }
    private fun onHandletimeout() { /* platform callback */ }
    private fun onRetrysecurityaccess() { /* platform callback */ }
}