
// GENERATED CODE — DO NOT EDIT
// Source: resources/234/test234_child1.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test234

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test234Child1State : State {
    data object Sub0 : Test234Child1State
    data object SubFinal2 : Test234Child1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test234Child1Event : Event {
    sealed interface Error : Test234Child1Event {
        data object Execution : Error
    }
    data object Timeout : Test234Child1Event
}
// --- State Machine (W3C SCXML) ---

class Test234Child1StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test234Child1State, Test234Child1Event>(scriptEngine) {

    override val initialState: Test234Child1State = Test234Child1State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test234Child1State? = when (stateId) {
        "sub0" -> Test234Child1State.Sub0
        "subFinal2" -> Test234Child1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test234Child1State): String = when (state) {
        is Test234Child1State.Sub0 -> "sub0"
        is Test234Child1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test234Child1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test234Child1State): Int = when (state) {
        is Test234Child1State.Sub0 -> 0
        is Test234Child1State.SubFinal2 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test234Child1State,
        event: Test234Child1Event
    ): TransitionResult<Test234Child1State> = when (state) {
        is Test234Child1State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test234Child1Event
    ): TransitionResult<Test234Child1State> = when {
        event is Test234Child1Event.Timeout -> TransitionResult.External(Test234Child1State.SubFinal2, Test234Child1State.Sub0)

        else -> TransitionResult.Ignored
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test234Child1State) {
        when (state) {
            is Test234Child1State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("__send_0", 2000L, Test234Child1Event.Timeout)
            }
            is Test234Child1State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test234Child1State) {
        when (state) {
            is Test234Child1State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test234Child1State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test234Child1State,
        event: Test234Child1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
