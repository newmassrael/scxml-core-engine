
// GENERATED CODE — DO NOT EDIT
// Source: resources/422/test422_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422Child0State : State {
    data object Sub0 : Test422Child0State
    data object SubFinal0 : Test422Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422Child0Event : Event {
    sealed interface Error : Test422Child0Event {
        data object Execution : Error
    }
    data object InvokeS1 : Test422Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test422Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test422Child0State, Test422Child0Event>(scriptEngine) {

    override val initialState: Test422Child0State = Test422Child0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422Child0State? = when (stateId) {
        "sub0" -> Test422Child0State.Sub0
        "subFinal0" -> Test422Child0State.SubFinal0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422Child0State): String = when (state) {
        is Test422Child0State.Sub0 -> "sub0"
        is Test422Child0State.SubFinal0 -> "subFinal0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422Child0State): Int = when (state) {
        is Test422Child0State.Sub0 -> 0
        is Test422Child0State.SubFinal0 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422Child0Event? = when (name) {
        "error.execution" -> Test422Child0Event.Error.Execution
        "invokeS1" -> Test422Child0Event.InvokeS1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422Child0Event): String? = when (event) {
        is Test422Child0Event.Error.Execution -> "error.execution"
        is Test422Child0Event.InvokeS1 -> "invokeS1"
        // Kotlin `when` expression exhaustiveness: a child machine that
        // inherits the override (has_parent_communication path) but
        // declares no events of its own produces an empty sealed
        // hierarchy, and `when (event)` without `else` fails to compile.
        // The branch is redundant on non-empty hierarchies but harmless.
        else -> null
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test422Child0State,
        event: Test422Child0Event
    ): TransitionResult<Test422Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422Child0State
    ): TransitionResult<Test422Child0State> = when (state) {
        is Test422Child0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test422Child0State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422Child0State.SubFinal0, Test422Child0State.Sub0)
    }

    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test422Child0State) {
        when (state) {
            is Test422Child0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS1", "")
            }
            is Test422Child0State.SubFinal0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal0")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test422Child0State) {
        when (state) {
            is Test422Child0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test422Child0State.SubFinal0 -> {
                activeStateIds.remove("subFinal0")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test422Child0State,
        event: Test422Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
