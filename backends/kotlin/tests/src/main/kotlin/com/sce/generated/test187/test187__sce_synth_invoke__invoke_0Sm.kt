// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test187

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test187SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test187SceSynthInvokeInvoke0State
    data object SubFinal : Test187SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test187SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test187SceSynthInvokeInvoke0Event
    sealed interface Error : Test187SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test187SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test187SceSynthInvokeInvoke0State, Test187SceSynthInvokeInvoke0Event>() {

    override val initialState: Test187SceSynthInvokeInvoke0State = Test187SceSynthInvokeInvoke0State.Sub0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test187SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test187SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test187SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test187SceSynthInvokeInvoke0State): String = when (state) {
        is Test187SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test187SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test187SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test187SceSynthInvokeInvoke0State): Int = when (state) {
        is Test187SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test187SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test187SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test187SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test187SceSynthInvokeInvoke0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test187SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test187SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test187SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test187SceSynthInvokeInvoke0State,
        event: Test187SceSynthInvokeInvoke0Event
    ): TransitionResult<Test187SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test187SceSynthInvokeInvoke0State
    ): TransitionResult<Test187SceSynthInvokeInvoke0State> = when (state) {
        is Test187SceSynthInvokeInvoke0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test187SceSynthInvokeInvoke0State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test187SceSynthInvokeInvoke0State.SubFinal, Test187SceSynthInvokeInvoke0State.Sub0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test187SceSynthInvokeInvoke0State, pathChild: Test187SceSynthInvokeInvoke0State?) {
        when (state) {
            is Test187SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test187): Delayed send to parent — cancelled when child stops
            scheduleParentSend("__send_0", 500L, "childToParent")
            }
            is Test187SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:10 :: subFinal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test187SceSynthInvokeInvoke0State) {
        when (state) {
            is Test187SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                activeStateIds.remove("sub0")
            }
            is Test187SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:10 :: subFinal :: _state_body
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test187__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test187SceSynthInvokeInvoke0State,
        event: Test187SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
