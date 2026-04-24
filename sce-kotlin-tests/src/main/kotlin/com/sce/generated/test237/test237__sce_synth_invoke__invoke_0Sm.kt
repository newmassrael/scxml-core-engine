
// GENERATED CODE — DO NOT EDIT
// Source: resources/237/test237__sce_synth_invoke__invoke_0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test237

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test237SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test237SceSynthInvokeInvoke0State
    data object SubFinal : Test237SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test237SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test237SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Timeout : Test237SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test237SceSynthInvokeInvoke0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test237SceSynthInvokeInvoke0State, Test237SceSynthInvokeInvoke0Event>(scriptEngine) {

    override val initialState: Test237SceSynthInvokeInvoke0State = Test237SceSynthInvokeInvoke0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test237SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test237SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test237SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test237SceSynthInvokeInvoke0State): String = when (state) {
        is Test237SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test237SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test237SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test237SceSynthInvokeInvoke0State): Int = when (state) {
        is Test237SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test237SceSynthInvokeInvoke0State.SubFinal -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test237SceSynthInvokeInvoke0State,
        event: Test237SceSynthInvokeInvoke0Event
    ): TransitionResult<Test237SceSynthInvokeInvoke0State> = when (state) {
        is Test237SceSynthInvokeInvoke0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test237SceSynthInvokeInvoke0Event
    ): TransitionResult<Test237SceSynthInvokeInvoke0State> = when {
        event is Test237SceSynthInvokeInvoke0Event.Timeout -> TransitionResult.External(Test237SceSynthInvokeInvoke0State.SubFinal, Test237SceSynthInvokeInvoke0State.Sub0)

        else -> TransitionResult.Ignored
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test237SceSynthInvokeInvoke0State) {
        when (state) {
            is Test237SceSynthInvokeInvoke0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("__send_0", 2000L, Test237SceSynthInvokeInvoke0Event.Timeout)
            }
            is Test237SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test237SceSynthInvokeInvoke0State) {
        when (state) {
            is Test237SceSynthInvokeInvoke0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test237SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test237SceSynthInvokeInvoke0State,
        event: Test237SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
