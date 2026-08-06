// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 9a73684207dcf4d7691a44d8d97d6208a949b25e3e8615da011239d058ccfc77
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test252__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test252

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test252SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test252SceSynthInvokeInvoke0State
    data object SubFinal : Test252SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test252SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test252SceSynthInvokeInvoke0Event
    sealed interface Error : Test252SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Timeout : Test252SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test252SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test252SceSynthInvokeInvoke0State, Test252SceSynthInvokeInvoke0Event>() {

    override val initialState: Test252SceSynthInvokeInvoke0State = Test252SceSynthInvokeInvoke0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test252SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test252SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test252SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test252SceSynthInvokeInvoke0State): String = when (state) {
        is Test252SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test252SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test252SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test252SceSynthInvokeInvoke0State): Int = when (state) {
        is Test252SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test252SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test252SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test252SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test252SceSynthInvokeInvoke0Event.Error.Execution
        "timeout" -> Test252SceSynthInvokeInvoke0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test252SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test252SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test252SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test252SceSynthInvokeInvoke0Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test252SceSynthInvokeInvoke0State,
        event: Test252SceSynthInvokeInvoke0Event
    ): TransitionResult<Test252SceSynthInvokeInvoke0State> = when (state) {
        is Test252SceSynthInvokeInvoke0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test252SceSynthInvokeInvoke0Event
    ): TransitionResult<Test252SceSynthInvokeInvoke0State> = when {
        event is Test252SceSynthInvokeInvoke0Event.Timeout -> TransitionResult.External(Test252SceSynthInvokeInvoke0State.SubFinal, Test252SceSynthInvokeInvoke0State.Sub0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test252__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test252SceSynthInvokeInvoke0State) {
        when (state) {
            is Test252SceSynthInvokeInvoke0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("__send_0", 500L, Test252SceSynthInvokeInvoke0Event.Timeout)
            }
            is Test252SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test252__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test252SceSynthInvokeInvoke0State) {
        when (state) {
            is Test252SceSynthInvokeInvoke0State.Sub0 -> {
                activeStateIds.remove("sub0")


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test252SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test252__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test252SceSynthInvokeInvoke0State,
        event: Test252SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
