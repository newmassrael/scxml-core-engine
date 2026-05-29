// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020098

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422SceSynthInvokeInvoke2State : State {
    data object Sub2 : Test422SceSynthInvokeInvoke2State
    data object SubFinal2 : Test422SceSynthInvokeInvoke2State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422SceSynthInvokeInvoke2Event : Event {
    sealed interface Error : Test422SceSynthInvokeInvoke2Event {
        data object Execution : Error
    }
    data object InvokeS12 : Test422SceSynthInvokeInvoke2Event
}
// --- State Machine (W3C SCXML) ---

class Test422SceSynthInvokeInvoke2StateMachine(
) : StateMachineEngine<Test422SceSynthInvokeInvoke2State, Test422SceSynthInvokeInvoke2Event>() {

    override val initialState: Test422SceSynthInvokeInvoke2State = Test422SceSynthInvokeInvoke2State.Sub2



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422SceSynthInvokeInvoke2State? = when (stateId) {
        "sub2" -> Test422SceSynthInvokeInvoke2State.Sub2
        "subFinal2" -> Test422SceSynthInvokeInvoke2State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422SceSynthInvokeInvoke2State): String = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> "sub2"
        is Test422SceSynthInvokeInvoke2State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422SceSynthInvokeInvoke2State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422SceSynthInvokeInvoke2State): Int = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> 0
        is Test422SceSynthInvokeInvoke2State.SubFinal2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422SceSynthInvokeInvoke2Event? = when (name) {
        "error.execution" -> Test422SceSynthInvokeInvoke2Event.Error.Execution
        "invokeS12" -> Test422SceSynthInvokeInvoke2Event.InvokeS12
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422SceSynthInvokeInvoke2Event): String? = when (event) {
        is Test422SceSynthInvokeInvoke2Event.Error.Execution -> "error.execution"
        is Test422SceSynthInvokeInvoke2Event.InvokeS12 -> "invokeS12"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test422SceSynthInvokeInvoke2State,
        event: Test422SceSynthInvokeInvoke2Event
    ): TransitionResult<Test422SceSynthInvokeInvoke2State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422SceSynthInvokeInvoke2State
    ): TransitionResult<Test422SceSynthInvokeInvoke2State> = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> processNullSub2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub2(
    ): TransitionResult<Test422SceSynthInvokeInvoke2State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422SceSynthInvokeInvoke2State.SubFinal2, Test422SceSynthInvokeInvoke2State.Sub2)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3
    override fun onEntry(state: Test422SceSynthInvokeInvoke2State) {
        when (state) {
            is Test422SceSynthInvokeInvoke2State.Sub2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub2")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS12", "")
            }
            is Test422SceSynthInvokeInvoke2State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3
    override fun onExit(state: Test422SceSynthInvokeInvoke2State) {
        when (state) {
            is Test422SceSynthInvokeInvoke2State.Sub2 -> {
                activeStateIds.remove("sub2")
            }
            is Test422SceSynthInvokeInvoke2State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3
    override fun executeTransitionActions(
        source: Test422SceSynthInvokeInvoke2State,
        event: Test422SceSynthInvokeInvoke2Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
