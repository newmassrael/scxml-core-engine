// GENERATED CODE — DO NOT EDIT
// Source: resources/422/test422_child2.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test422

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test422Child2State : State {
    data object Sub2 : Test422Child2State
    data object SubFinal2 : Test422Child2State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422Child2Event : Event {
    sealed interface Error : Test422Child2Event {
        data object Execution : Error
    }
    data object InvokeS12 : Test422Child2Event
}
// --- State Machine (W3C SCXML) ---

class Test422Child2StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test422Child2State, Test422Child2Event>(scriptEngine) {

    override val initialState: Test422Child2State = Test422Child2State.Sub2



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422Child2State? = when (stateId) {
        "sub2" -> Test422Child2State.Sub2
        "subFinal2" -> Test422Child2State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422Child2State): String = when (state) {
        is Test422Child2State.Sub2 -> "sub2"
        is Test422Child2State.SubFinal2 -> "subFinal2"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422Child2State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422Child2State): Int = when (state) {
        is Test422Child2State.Sub2 -> 0
        is Test422Child2State.SubFinal2 -> 1
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422Child2Event? = when (name) {
        "error.execution" -> Test422Child2Event.Error.Execution
        "invokeS12" -> Test422Child2Event.InvokeS12
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422Child2Event): String? = when (event) {
        is Test422Child2Event.Error.Execution -> "error.execution"
        is Test422Child2Event.InvokeS12 -> "invokeS12"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test422Child2State,
        event: Test422Child2Event
    ): TransitionResult<Test422Child2State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422Child2State
    ): TransitionResult<Test422Child2State> = when (state) {
        is Test422Child2State.Sub2 -> processNullSub2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub2(
    ): TransitionResult<Test422Child2State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422Child2State.SubFinal2, Test422Child2State.Sub2)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test422Child2State) {
        when (state) {
            is Test422Child2State.Sub2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub2")) return
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS12", "")
            }
            is Test422Child2State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test422Child2State) {
        when (state) {
            is Test422Child2State.Sub2 -> {
                activeStateIds.remove("sub2")
            }
            is Test422Child2State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test422Child2State,
        event: Test422Child2Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
