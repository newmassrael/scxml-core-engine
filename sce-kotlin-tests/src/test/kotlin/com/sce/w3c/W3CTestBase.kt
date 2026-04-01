// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// W3C SCXML Conformance Test Harness for Kotlin AOT State Machines

package com.sce.w3c

import com.sce.runtime.Event
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/**
 * Abstract base class for W3C SCXML conformance tests.
 *
 * Each generated test extends this class and provides:
 *   - [createStateMachine]: Factory for the generated StateMachineEngine
 *   - [expectedPassState]: The "pass" final state object
 *
 * The test harness starts the state machine, waits for it to reach
 * a final state, and asserts the final state is the expected "pass" state.
 *
 * W3C SCXML test convention: Each test has "pass" and "fail" final states.
 *
 * Threading: The SM microstep loop runs on Dispatchers.Default.
 * [isInFinalState] is guaranteed to be set only after [currentState]
 * reflects the final state (see [StateMachineEngine.markFinalStateReached]).
 *
 * @param S State sealed interface type
 * @param E Event sealed interface type
 */
abstract class W3CTestBase<S : State, E : Event> {

    abstract fun createStateMachine(): StateMachineEngine<S, E>
    abstract val expectedPassState: S

    /**
     * Timeout for state machine completion in milliseconds.
     * Override for tests with delayed sends that need more time.
     */
    open val timeoutMs: Long = 5000L

    @Test
    fun testW3CConformance() = runBlocking {
        val sm = createStateMachine()
        sm.start(this)

        try {
            withTimeout(timeoutMs) {
                // Poll until the SM reaches a final state.
                // isInFinalState is @Volatile and guaranteed to be set
                // only after currentState.value reflects the final state
                // (enforced by markFinalStateReached() deferred flush).
                while (!sm.isInFinalState) {
                    delay(1)
                }
            }

            assertEquals(
                expectedPassState,
                sm.currentState.value,
                "W3C conformance failed: expected Pass but reached ${sm.currentState.value}"
            )
        } finally {
            sm.stop()
        }
    }
}
