// GENERATED CODE — DO NOT EDIT
// Source: resources/347/test347_machineName.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test347

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test347MachineNameState : State {
    data object Sub0 : Test347MachineNameState
    data object SubFinal : Test347MachineNameState
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test347MachineNameEvent : Event {
    data object ChildToParent : Test347MachineNameEvent
    sealed interface Error : Test347MachineNameEvent {
        data object Execution : Error
    }
    data object ParentToChild : Test347MachineNameEvent
}
// --- State Machine (W3C SCXML) ---

class Test347MachineNameStateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test347MachineNameState, Test347MachineNameEvent>(scriptEngine) {

    override val initialState: Test347MachineNameState = Test347MachineNameState.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test347MachineNameState? = when (stateId) {
        "sub0" -> Test347MachineNameState.Sub0
        "subFinal" -> Test347MachineNameState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test347MachineNameState): String = when (state) {
        is Test347MachineNameState.Sub0 -> "sub0"
        is Test347MachineNameState.SubFinal -> "subFinal"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test347MachineNameState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test347MachineNameState): Int = when (state) {
        is Test347MachineNameState.Sub0 -> 0
        is Test347MachineNameState.SubFinal -> 1
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test347MachineNameEvent? = when (name) {
        "childToParent" -> Test347MachineNameEvent.ChildToParent
        "error.execution" -> Test347MachineNameEvent.Error.Execution
        "parentToChild" -> Test347MachineNameEvent.ParentToChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test347MachineNameEvent): String? = when (event) {
        is Test347MachineNameEvent.ChildToParent -> "childToParent"
        is Test347MachineNameEvent.Error.Execution -> "error.execution"
        is Test347MachineNameEvent.ParentToChild -> "parentToChild"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test347MachineNameState,
        event: Test347MachineNameEvent
    ): TransitionResult<Test347MachineNameState> = when (state) {
        is Test347MachineNameState.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test347MachineNameEvent
    ): TransitionResult<Test347MachineNameState> = when {
        event is Test347MachineNameEvent.ParentToChild -> TransitionResult.External(Test347MachineNameState.SubFinal, Test347MachineNameState.Sub0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test347MachineNameState) {
        when (state) {
            is Test347MachineNameState.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test347MachineNameState.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test347MachineNameState) {
        when (state) {
            is Test347MachineNameState.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test347MachineNameState.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test347MachineNameState,
        event: Test347MachineNameEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
