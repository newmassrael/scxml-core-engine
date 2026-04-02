// GENERATED CODE — DO NOT EDIT
// Source: resources/522/test522.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test522

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test522State : State {
    data object Fail : Test522State
    data object Pass : Test522State
    data object S0 : Test522State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test522Event : Event {
    sealed interface Error : Test522Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Test : Test522Event
    data object Timeout : Test522Event
}
// --- State Machine (W3C SCXML) ---

class Test522StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test522State, Test522Event>(scriptEngine) {

    override val initialState: Test522State = Test522State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test522State? = when (stateId) {
        "fail" -> Test522State.Fail
        "pass" -> Test522State.Pass
        "s0" -> Test522State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test522State): String = when (state) {
        is Test522State.Fail -> "fail"
        is Test522State.Pass -> "pass"
        is Test522State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test522State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test522State): Int = when (state) {
        is Test522State.Fail -> 2
        is Test522State.Pass -> 1
        is Test522State.S0 -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test522State,
        event: Test522Event
    ): TransitionResult<Test522State> = when (state) {
        is Test522State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test522Event
    ): TransitionResult<Test522State> = when {
        event is Test522Event.Timeout -> TransitionResult.External(Test522State.Fail, Test522State.S0)

        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test522Event.Error || event is Test522Event.Error.Execution) -> TransitionResult.External(Test522State.Fail, Test522State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test522State.Pass)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test522State) {
        when (state) {
            is Test522State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test522State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test522State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 30000L, Test522Event.Timeout)
            send(Test522Event.Test, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test522State) {
        when (state) {
            is Test522State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test522State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test522State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test522State,
        event: Test522Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
