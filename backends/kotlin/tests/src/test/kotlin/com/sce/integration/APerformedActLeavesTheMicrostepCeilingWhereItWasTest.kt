// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A macrostep stopped at the microstep ceiling gets a fresh budget where an
// event comes off the external queue, and nowhere else (ARCHITECTURE.md
// "External-Event Budget"; the C++ and Rust engines release it at the same one
// point). A scheduled act the engine performs when its delay elapses, such as a
// delayed host-served `<send>`, is not such an event: the chain the ceiling
// stopped stays stopped through it, in both of this engine's driving modes.
//
// The coroutine mode used to release the ceiling before the macrostep an act
// started, so the same document re-ran its endless chain there, and was cut a
// second time, where the synchronous mode did not (docs/adr/0084, D2 as amended
// 2026-11-16).
//
// The machine is written by hand: one state whose `spin` raises `spin` again,
// a chain that never ends, and whose entry arms a delayed host-served send.

package com.sce.integration

import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.HistoryId
import com.sce.runtime.InboxInput
import com.sce.runtime.ManualClock
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

private sealed interface ActCeilingState : State {
    data object Spinning : ActCeilingState
}

private sealed interface ActCeilingEvent : Event {
    data object Spin : ActCeilingEvent
}

/**
 * `spinning` takes `spin` by raising `spin` again, so a delivered `spin` is a
 * chain the microstep ceiling has to stop. Its entry arms a host-served send of
 * [ACT_TYPE], delayed by [actDelayMs].
 */
private class ActCeilingMachine(
    private val actDelayMs: Long,
) : StateMachineEngine<ActCeilingState, ActCeilingEvent>() {
    override val initialState: ActCeilingState = ActCeilingState.Spinning

    override fun firstEnabledTransition(
        state: ActCeilingState,
        event: ActCeilingEvent?,
    ): EnabledTransition<ActCeilingState, HistoryId>? =
        when (event) {
            null -> null
            ActCeilingEvent.Spin -> EnabledTransition<ActCeilingState, HistoryId>(
                state,
                emptyList(),
                SPIN,
                hasActions = true,
                isInternal = false,
            )
        }

    override fun onEntry(state: ActCeilingState, isDefaultEntry: Boolean) {
        scheduleHostSend("act", actDelayMs, HostSendRequest(processorType = ACT_TYPE, sendId = "act"))
    }

    override fun onExit(state: ActCeilingState) {}

    override fun executeTransitionContent(source: ActCeilingState, transitionIndex: Int) {
        if (transitionIndex == SPIN) raiseInternal(ActCeilingEvent.Spin)
    }

    override fun resolveState(stateId: String): ActCeilingState? =
        if (stateId == "spinning") ActCeilingState.Spinning else null

    override fun stateIdOf(state: ActCeilingState): String = "spinning"

    override fun resolveEventByName(name: String): ActCeilingEvent? =
        if (name == "spin") ActCeilingEvent.Spin else null

    override fun eventNameOf(event: ActCeilingEvent): String = "spin"

    companion object {
        const val ACT_TYPE = "x-sce-probe-act"
        const val SPIN = 0
    }
}

@DisplayName("A performed act leaves the microstep ceiling where it was")
class APerformedActLeavesTheMicrostepCeilingWhereItWasTest {

    private val boundSeconds = 10L

    @Test
    fun aCoroutineModeActDoesNotGiveAStoppedChainASecondBudget() {
        val sm = ActCeilingMachine(actDelayMs = 300)
        val performed = CountDownLatch(1)
        sm.registerEventProcessor(ActCeilingMachine.ACT_TYPE) {
            performed.countDown()
            emptyList()
        }
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            // Posted before the run, which keeps it: the loop's first drain takes
            // `spin`, long before the act is due.
            assertTrue(sm.inbox.post(InboxInput.ExternalEvent(ActCeilingEvent.Spin)))
            sm.start(scope)
            assertTrue(performed.await(boundSeconds, TimeUnit.SECONDS), "the delayed act was never performed")
        } finally {
            // The stop waits for the macrostep the act started, so the count below
            // is read after it.
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
        assertEquals(
            1,
            sm.truncatedMacrosteps(),
            "the act's macrostep re-ran the chain the ceiling had stopped, with a budget of its own: " +
                "only an event taken off the external queue releases the ceiling"
        )
    }

    @Test
    fun aSynchronousModeActDoesNotGiveAStoppedChainASecondBudget() {
        val sm = ActCeilingMachine(actDelayMs = 300)
        var performed = 0
        sm.registerEventProcessor(ActCeilingMachine.ACT_TYPE) {
            performed++
            emptyList()
        }
        sm.clock = ManualClock()
        sm.initialize()
        try {
            sm.send(ActCeilingEvent.Spin)
            sm.tick()
            assertEquals(1, sm.truncatedMacrosteps(), "`spin` is a chain that never ends; the ceiling should stop it once")
            sm.advanceTimeMs(300)
            assertEquals(1, performed, "the delayed act was not performed when its delay elapsed")
            assertEquals(
                1,
                sm.truncatedMacrosteps(),
                "the act's macrostep re-ran the chain the ceiling had stopped, with a budget of its own"
            )
        } finally {
            sm.cleanup()
        }
    }
}
