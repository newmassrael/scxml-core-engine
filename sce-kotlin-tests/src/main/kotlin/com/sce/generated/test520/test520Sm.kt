// GENERATED CODE — DO NOT EDIT
// Source: resources/520/test520.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test520

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test520State : State {
    data object Fail : Test520State
    data object Pass : Test520State
    data object S0 : Test520State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test520Event : Event {
    data object Empty : Test520Event
    sealed interface HTTP : Test520Event {
        data object POST : HTTP
    }
    sealed interface Error : Test520Event {
        data object Execution : Error
    }
    data object Timeout : Test520Event
}
// --- State Machine (W3C SCXML) ---

class Test520StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test520State, Test520Event>(scriptEngine) {

    override val initialState: Test520State = Test520State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test520State? = when (stateId) {
        "fail" -> Test520State.Fail
        "pass" -> Test520State.Pass
        "s0" -> Test520State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test520State): String = when (state) {
        is Test520State.Fail -> "fail"
        is Test520State.Pass -> "pass"
        is Test520State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test520State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test520State): Int = when (state) {
        is Test520State.Fail -> 2
        is Test520State.Pass -> 1
        is Test520State.S0 -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test520State,
        event: Test520Event
    ): TransitionResult<Test520State> = when (state) {
        is Test520State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test520Event
    ): TransitionResult<Test520State> = when {
        event is Test520Event.HTTP.POST -> TransitionResult.External(Test520State.Pass, Test520State.S0)

        event is Test520Event.HTTP.POST -> TransitionResult.External(Test520State.Pass, Test520State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test520State.Fail, Test520State.S0)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test520State) {
        when (state) {
            is Test520State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test520State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test520State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 30000L, Test520Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test520State) {
        when (state) {
            is Test520State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test520State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test520State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test520State,
        event: Test520Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
