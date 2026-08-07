// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7d180dffdd955c10062343fb76305c7a80a95112d21da2591e0f0959805b08ad
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test207

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test207SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test207SceSynthInvokeInvoke0State
    data object SubFinal : Test207SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test207SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test207SceSynthInvokeInvoke0Event
    sealed interface Error : Test207SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Event1 : Test207SceSynthInvokeInvoke0Event
    data object Event2 : Test207SceSynthInvokeInvoke0Event
    data object Fail : Test207SceSynthInvokeInvoke0Event
    data object Pass : Test207SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test207SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test207SceSynthInvokeInvoke0State, Test207SceSynthInvokeInvoke0Event>() {

    override val initialState: Test207SceSynthInvokeInvoke0State = Test207SceSynthInvokeInvoke0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test207SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test207SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test207SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test207SceSynthInvokeInvoke0State): String = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test207SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test207SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test207SceSynthInvokeInvoke0State): Int = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test207SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test207SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test207SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test207SceSynthInvokeInvoke0Event.Error.Execution
        "event1" -> Test207SceSynthInvokeInvoke0Event.Event1
        "event2" -> Test207SceSynthInvokeInvoke0Event.Event2
        "fail" -> Test207SceSynthInvokeInvoke0Event.Fail
        "pass" -> Test207SceSynthInvokeInvoke0Event.Pass
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test207SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test207SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test207SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test207SceSynthInvokeInvoke0Event.Event1 -> "event1"
        is Test207SceSynthInvokeInvoke0Event.Event2 -> "event2"
        is Test207SceSynthInvokeInvoke0Event.Fail -> "fail"
        is Test207SceSynthInvokeInvoke0Event.Pass -> "pass"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test207SceSynthInvokeInvoke0State,
        event: Test207SceSynthInvokeInvoke0Event
    ): TransitionResult<Test207SceSynthInvokeInvoke0State> = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test207SceSynthInvokeInvoke0Event
    ): TransitionResult<Test207SceSynthInvokeInvoke0State> = when {
        event is Test207SceSynthInvokeInvoke0Event.Event1 -> TransitionResult.External(Test207SceSynthInvokeInvoke0State.SubFinal, Test207SceSynthInvokeInvoke0State.Sub0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test207SceSynthInvokeInvoke0State.SubFinal, Test207SceSynthInvokeInvoke0State.Sub0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test207SceSynthInvokeInvoke0State) {
        when (state) {
            is Test207SceSynthInvokeInvoke0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("foo", 1000L, Test207SceSynthInvokeInvoke0Event.Event1)


            scheduleSend("__send_2", 1500L, Test207SceSynthInvokeInvoke0Event.Event2)


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test207SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test207SceSynthInvokeInvoke0State) {
        when (state) {
            is Test207SceSynthInvokeInvoke0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test207SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test207SceSynthInvokeInvoke0State,
        event: Test207SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> when {
            event is Test207SceSynthInvokeInvoke0Event.Event1 -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("pass", "")
            }
            event != null -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("fail", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
