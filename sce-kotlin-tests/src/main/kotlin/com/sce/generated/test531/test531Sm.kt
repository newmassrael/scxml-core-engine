// GENERATED CODE — DO NOT EDIT
// Source: resources/531/test531.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test531

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test531State : State {
    data object Fail : Test531State
    data object Pass : Test531State
    data object S0 : Test531State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test531Event : Event {
    sealed interface Error : Test531Event {
        data object Execution : Error
    }
    data object Test : Test531Event
    data object Timeout : Test531Event
}
// --- State Machine (W3C SCXML) ---

class Test531StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test531State, Test531Event>(scriptEngine) {

    override val initialState: Test531State = Test531State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test531State? = when (stateId) {
        "fail" -> Test531State.Fail
        "pass" -> Test531State.Pass
        "s0" -> Test531State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test531State): String = when (state) {
        is Test531State.Fail -> "fail"
        is Test531State.Pass -> "pass"
        is Test531State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test531State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test531State): Int = when (state) {
        is Test531State.Fail -> 2
        is Test531State.Pass -> 1
        is Test531State.S0 -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test531State,
        event: Test531Event
    ): TransitionResult<Test531State> = when (state) {
        is Test531State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test531Event
    ): TransitionResult<Test531State> = when {
        event is Test531Event.Test -> TransitionResult.External(Test531State.Pass, Test531State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test531State.Fail, Test531State.S0)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 3000L, Test531Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test531State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test531State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test531State,
        event: Test531Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
