// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 50d6eb36f321e50c2a6e5457f0a900b925f832ee57619f9b6a33cf22bd75d4e1
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test229__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test229

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test229SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test229SceSynthInvokeInvoke0State
    data object SubFinal : Test229SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test229SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test229SceSynthInvokeInvoke0Event
    sealed interface Error : Test229SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object EventReceived : Test229SceSynthInvokeInvoke0Event
    data object Timeout : Test229SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test229SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test229SceSynthInvokeInvoke0State, Test229SceSynthInvokeInvoke0Event>() {

    override val initialState: Test229SceSynthInvokeInvoke0State = Test229SceSynthInvokeInvoke0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test229SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test229SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test229SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test229SceSynthInvokeInvoke0State): String = when (state) {
        is Test229SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test229SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test229SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test229SceSynthInvokeInvoke0State): Int = when (state) {
        is Test229SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test229SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test229SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test229SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test229SceSynthInvokeInvoke0Event.Error.Execution
        "eventReceived" -> Test229SceSynthInvokeInvoke0Event.EventReceived
        "timeout" -> Test229SceSynthInvokeInvoke0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test229SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test229SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test229SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test229SceSynthInvokeInvoke0Event.EventReceived -> "eventReceived"
        is Test229SceSynthInvokeInvoke0Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test229SceSynthInvokeInvoke0State,
        event: Test229SceSynthInvokeInvoke0Event
    ): TransitionResult<Test229SceSynthInvokeInvoke0State> = when (state) {
        is Test229SceSynthInvokeInvoke0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test229SceSynthInvokeInvoke0Event
    ): TransitionResult<Test229SceSynthInvokeInvoke0State> = when {
        event is Test229SceSynthInvokeInvoke0Event.ChildToParent -> TransitionResult.External(Test229SceSynthInvokeInvoke0State.SubFinal, Test229SceSynthInvokeInvoke0State.Sub0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test229SceSynthInvokeInvoke0State.SubFinal, Test229SceSynthInvokeInvoke0State.Sub0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test229__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test229SceSynthInvokeInvoke0State) {
        when (state) {
            is Test229SceSynthInvokeInvoke0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")


            scheduleSend("__send_2", 3000L, Test229SceSynthInvokeInvoke0Event.Timeout)
            }
            is Test229SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test229__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test229SceSynthInvokeInvoke0State) {
        when (state) {
            is Test229SceSynthInvokeInvoke0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test229SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test229__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test229SceSynthInvokeInvoke0State,
        event: Test229SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        is Test229SceSynthInvokeInvoke0State.Sub0 -> when {
            event is Test229SceSynthInvokeInvoke0Event.ChildToParent -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("eventReceived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
