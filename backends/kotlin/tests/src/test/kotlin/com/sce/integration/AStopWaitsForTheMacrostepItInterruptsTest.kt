// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D + 6.2.4 — the coroutine mode's `stop()` does not run
// beside a macrostep. Kotlin AOT path.
//
// A coroutine-mode engine runs its macrosteps on `Dispatchers.Default`, and its
// state (the configuration, the queues, the delayed sends) is confined to that
// one coroutine: the engine says so wherever it touches them. `stop()` is called
// from the host's thread, and it used to cancel the coroutine and then clear that
// same state at once. Cancelling does not interrupt a macrostep that is running,
// so for as long as one was, two threads held the same non-thread-safe list.
//
// Measured on the hosted runner (2026-10-02, run 37027519407, the Kotlin W3C
// job): `DelayedHostSend.aHostServedSendIsPerformedInCoroutineMode` failed with
//
//     NullPointerException: Cannot invoke "ScheduledSendEntry.getFireTimeMs()"
//     because "it" is null    at queueScheduledSend ... TimSort
//
// which is `scheduledSends.clear()` on the stopping thread landing in the middle
// of the engine thread's sort of the same list. That is a timing accident; what
// is stated here is the invariant it is an instance of, and it is stated without
// one: a handler the engine calls from inside a macrostep is held there, `stop()`
// is called from another thread, and it must not return until that macrostep has
// ended.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// (canonical, shared with the Rust / C++ / C11 / Go / Python channels). Its
// delayed host-served send is performed by the engine's own loop, which is where
// the handler is held.

package com.sce.integration

import com.sce.integration.statechart_delayed_host_send.StatechartDelayedHostSendStateMachine
import com.sce.runtime.StateMachineEngine
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML Appendix D — `stop()` and the macrostep it interrupts.
@DisplayName("AStopWaitsForTheMacrostepItInterrupts — W3C SCXML Appendix D")
class AStopWaitsForTheMacrostepItInterruptsTest {

    // The type the fixture was compiled for; the same string
    // `scripts/regen_host_processor_kotlin.sh` passes to `--host-processor`.
    private val declaredType = "x-sce-host"

    /**
     * Bounded far above anything this waits for, so a stall names the stage that
     * stalled instead of surfacing as the runner's own per-test limit.
     */
    private val boundSeconds = 8L

    @Test
    fun stopReturnsOnlyOnceTheMacrostepItInterruptedHasEnded() = runBlocking {
        val inHandler = CountDownLatch(1)
        val release = CountDownLatch(1)
        val sm = StatechartDelayedHostSendStateMachine()
        sm.registerEventProcessor(declaredType) {
            inHandler.countDown()
            // The handler is the engine's own call, made from inside the
            // macrostep that performs the delayed send: while it is held the
            // engine thread is inside that macrostep and cannot be outside it.
            release.await(boundSeconds, TimeUnit.SECONDS)
            listOf(StateMachineEngine.HostSendResponse("turn.done"))
        }
        sm.start(this)

        assertTrue(
            inHandler.await(boundSeconds, TimeUnit.SECONDS),
            "the delayed host-served send was never performed in coroutine mode",
        )

        // From a thread of its own, as a host stops an engine it does not run on.
        val stopped = AtomicBoolean(false)
        val stopper = Thread {
            sm.stop()
            stopped.set(true)
        }
        stopper.start()
        try {
            // Long enough that a `stop()` that did not wait would have returned.
            stopper.join(400)
            assertFalse(
                stopped.get(),
                "stop() returned while the engine was still inside a macrostep: " +
                    "it tore the machine down beside the thread running it",
            )
        } finally {
            release.countDown()
        }
        stopper.join(boundSeconds * 1000)
        assertTrue(stopped.get(), "stop() never returned once the macrostep had ended")
    }
}
