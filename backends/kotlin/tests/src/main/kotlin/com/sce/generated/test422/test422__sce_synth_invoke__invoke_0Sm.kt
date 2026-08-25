// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c11ce025286de32d15ba70522b50fb24cf722356167a9d021470bd1434f2dd9a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test422SceSynthInvokeInvoke0State
    data object SubFinal0 : Test422SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test422SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object InvokeS1 : Test422SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test422SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test422SceSynthInvokeInvoke0State, Test422SceSynthInvokeInvoke0Event>() {

    override val initialState: Test422SceSynthInvokeInvoke0State = Test422SceSynthInvokeInvoke0State.Sub0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test422SceSynthInvokeInvoke0State.Sub0
        "subFinal0" -> Test422SceSynthInvokeInvoke0State.SubFinal0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422SceSynthInvokeInvoke0State): String = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test422SceSynthInvokeInvoke0State.SubFinal0 -> "subFinal0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422SceSynthInvokeInvoke0State): Int = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test422SceSynthInvokeInvoke0State.SubFinal0 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test422SceSynthInvokeInvoke0Event.Error.Execution
        "invokeS1" -> Test422SceSynthInvokeInvoke0Event.InvokeS1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test422SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test422SceSynthInvokeInvoke0Event.InvokeS1 -> "invokeS1"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test422SceSynthInvokeInvoke0State,
        event: Test422SceSynthInvokeInvoke0Event
    ): TransitionResult<Test422SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422SceSynthInvokeInvoke0State
    ): TransitionResult<Test422SceSynthInvokeInvoke0State> = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test422SceSynthInvokeInvoke0State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422SceSynthInvokeInvoke0State.SubFinal0, Test422SceSynthInvokeInvoke0State.Sub0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test422SceSynthInvokeInvoke0State, pathChild: Test422SceSynthInvokeInvoke0State?) {
        when (state) {
            is Test422SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS1", "")
            }
            is Test422SceSynthInvokeInvoke0State.SubFinal0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:10 :: subFinal0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal0")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test422SceSynthInvokeInvoke0State) {
        when (state) {
            is Test422SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                activeStateIds.remove("sub0")
            }
            is Test422SceSynthInvokeInvoke0State.SubFinal0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:10 :: subFinal0 :: _state_body
                activeStateIds.remove("subFinal0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test422SceSynthInvokeInvoke0State,
        event: Test422SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
