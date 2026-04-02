// GENERATED CODE — DO NOT EDIT
// Source: resources/403/test403a.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test403a

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test403aState : State {
    data object Fail : Test403aState
    data object Pass : Test403aState
    data object S0 : Test403aState
    data object S01 : Test403aState
    data object S02 : Test403aState
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test403aEvent : Event {
    sealed interface Error : Test403aEvent {
        data object Execution : Error
    }
    data object Event1 : Test403aEvent
    data object Event2 : Test403aEvent
    data object Timeout : Test403aEvent
}
// --- State Machine (W3C SCXML) ---

class Test403aStateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test403aState, Test403aEvent>(scriptEngine) {

    override val initialState: Test403aState = Test403aState.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test403aState.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test403aState): Test403aState? = when (state) {
        is Test403aState.S01 -> Test403aState.S0
        is Test403aState.S02 -> Test403aState.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test403aState): Test403aState = when (state) {
        is Test403aState.S0 -> Test403aState.S01
        else -> state
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test403aState,
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when (state) {
        is Test403aState.S0 -> processS0(event)
        is Test403aState.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test403aState.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Timeout -> TransitionResult.External(Test403aState.Fail)
        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.Fail)
        event is Test403aEvent.Event2 -> TransitionResult.External(Test403aState.Pass)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.S02)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test403aState.Fail)
    }

    private fun processS02(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.Fail)
        event is Test403aEvent.Event2 && false -> TransitionResult.External(Test403aState.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test403aState) {
        when (state) {
            is Test403aState.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403aState.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403aState.S0 -> {
            scheduleSend("__send_0", 1000L, Test403aEvent.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test403aState.S01)
            }
            is Test403aState.S01 -> {
            raiseInternal(Test403aEvent.Event1)
            }
            is Test403aState.S02 -> {
            raiseInternal(Test403aEvent.Event2)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test403aState) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test403aState,
        event: Test403aEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
