// GENERATED CODE — DO NOT EDIT
// Source: resources/412/test412.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test412

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test412State : State {
    data object Fail : Test412State
    data object Pass : Test412State
    data object S0 : Test412State
    data object S01 : Test412State
    data object S011 : Test412State
    data object S02 : Test412State
    data object S03 : Test412State
    data object S04 : Test412State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test412Event : Event {
    sealed interface Error : Test412Event {
        data object Execution : Error
    }
    data object Event1 : Test412Event
    data object Event2 : Test412Event
    data object Event3 : Test412Event
    data object Timeout : Test412Event
}
// --- State Machine (W3C SCXML) ---

class Test412StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test412State, Test412Event>(scriptEngine) {

    override val initialState: Test412State = Test412State.S011

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test412State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test412State): Test412State? = when (state) {
        is Test412State.S01 -> Test412State.S0
        is Test412State.S011 -> Test412State.S01
        is Test412State.S02 -> Test412State.S0
        is Test412State.S03 -> Test412State.S0
        is Test412State.S04 -> Test412State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test412State): Test412State = when (state) {
        is Test412State.S0 -> Test412State.S011
        is Test412State.S01 -> Test412State.S011
        else -> state
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test412State,
        event: Test412Event
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test412State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test412State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test412State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S04 -> {
            val result = processS04(event)
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

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test412State
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S011 -> processNullS011()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS011(
    ): TransitionResult<Test412State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test412State.S02)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Timeout -> TransitionResult.External(Test412State.Fail)
        event is Test412Event.Event1 -> TransitionResult.External(Test412State.Fail)
        event is Test412Event.Event2 -> TransitionResult.External(Test412State.Pass)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event1 -> TransitionResult.External(Test412State.S03)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail)
    }

    private fun processS03(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event2 -> TransitionResult.External(Test412State.S04)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail)
    }

    private fun processS04(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event3 -> TransitionResult.External(Test412State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test412State) {
        when (state) {
            is Test412State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.S0 -> {
            scheduleSend("__send_0", 1000L, Test412Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test412State.S01)
            }
            is Test412State.S01 -> {
            raiseInternal(Test412Event.Event1)
                // W3C SCXML 3.3.2: Execute initial transition content
            raiseInternal(Test412Event.Event2)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test412State.S011)
            }
            is Test412State.S011 -> {
            raiseInternal(Test412Event.Event3)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test412State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test412State,
        event: Test412Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
