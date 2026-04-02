// GENERATED CODE — DO NOT EDIT
// Source: resources/416/test416.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test416

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test416State : State {
    data object Fail : Test416State
    data object Pass : Test416State
    data object S1 : Test416State
    data object S11 : Test416State
    data object S111 : Test416State
    data object S11final : Test416State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test416Event : Event {
    sealed interface Done : Test416Event {
        sealed interface State : Done {
            data object S11 : State
        }
    }
    sealed interface Error : Test416Event {
        data object Execution : Error
    }
    data object Timeout : Test416Event
}
// --- State Machine (W3C SCXML) ---

class Test416StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test416State, Test416Event>(scriptEngine) {

    override val initialState: Test416State = Test416State.S111

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test416State.S1)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test416State): Test416State? = when (state) {
        is Test416State.S11 -> Test416State.S1
        is Test416State.S111 -> Test416State.S11
        is Test416State.S11final -> Test416State.S11
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test416State): Test416State = when (state) {
        is Test416State.S1 -> Test416State.S111
        is Test416State.S11 -> Test416State.S111
        else -> state
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test416State,
        event: Test416Event
    ): TransitionResult<Test416State> = when (state) {
        is Test416State.S1 -> processS1(event)
        is Test416State.S11 -> {
            val result = processS11(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test416State.S111 -> {
            val anc1 = processS11(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11final has no own event transitions)
        is Test416State.S11final -> {
            val anc1 = processS11(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test416State
    ): TransitionResult<Test416State> = when (state) {
        is Test416State.S111 -> processNullS111()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS111(
    ): TransitionResult<Test416State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test416State.S11final)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test416Event
    ): TransitionResult<Test416State> = when {
        event is Test416Event.Timeout -> TransitionResult.External(Test416State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS11(
        event: Test416Event
    ): TransitionResult<Test416State> = when {
        event is Test416Event.Done.State.S11 -> TransitionResult.External(Test416State.Pass)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test416State) {
        when (state) {
            is Test416State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test416State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test416State.S1 -> {
            scheduleSend("__send_0", 1000L, Test416Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test416State.S11)
            }
            is Test416State.S11 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test416State.S111)
            }
            is Test416State.S11final -> {
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test416Event.Done.State.S11, EventMetadata.platform())
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test416State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test416State,
        event: Test416Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
