// GENERATED CODE — DO NOT EDIT
// Source: resources/207/test207.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test207

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test207State : State {
    data object Fail : Test207State
    data object Pass : Test207State
    data object S0 : Test207State
    data object S01 : Test207State
    data object S02 : Test207State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test207Event : Event {
    sealed interface Cancel : Test207Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test207Event
    sealed interface Done : Test207Event {
        data object Invoke : Done
    }
    sealed interface Error : Test207Event {
        data object Execution : Error
    }
    data object Fail : Test207Event
    data object Pass : Test207Event
    data object Timeout : Test207Event
}
// --- State Machine (W3C SCXML) ---

class Test207StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test207State, Test207Event>(scriptEngine) {

    override val initialState: Test207State = Test207State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test207State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test207State): Test207State? = when (state) {
        is Test207State.S01 -> Test207State.S0
        is Test207State.S02 -> Test207State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test207State): Test207State = when (state) {
        is Test207State.S0 -> Test207State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test207Event? = when (name) {
        "cancel.invoke" -> Test207Event.Cancel.Invoke
        "childToParent" -> Test207Event.ChildToParent
        "done.invoke" -> Test207Event.Done.Invoke
        "error.execution" -> Test207Event.Error.Execution
        "fail" -> Test207Event.Fail
        "pass" -> Test207Event.Pass
        "timeout" -> Test207Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test207Event): String? = when (event) {
        is Test207Event.Cancel.Invoke -> "cancel.invoke"
        is Test207Event.ChildToParent -> "childToParent"
        is Test207Event.Done.Invoke -> "done.invoke"
        is Test207Event.Error.Execution -> "error.execution"
        is Test207Event.Fail -> "fail"
        is Test207Event.Pass -> "pass"
        is Test207Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test207State,
        event: Test207Event
    ): TransitionResult<Test207State> = when (state) {
        is Test207State.S01 -> processS01(event)
        is Test207State.S02 -> processS02(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS01(
        event: Test207Event
    ): TransitionResult<Test207State> = when {
        event is Test207Event.ChildToParent -> TransitionResult.External(Test207State.S02)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test207Event
    ): TransitionResult<Test207State> = when {
        event is Test207Event.Pass -> TransitionResult.External(Test207State.Pass)
        event is Test207Event.Fail -> TransitionResult.External(Test207State.Fail)
        event is Test207Event.Timeout -> TransitionResult.External(Test207State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test207State) {
        when (state) {
            is Test207State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test207State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test207State.S0 -> {
            scheduleSend("__send_0", 2000L, Test207Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test207Child0StateMachine(scriptEngine), false, Test207Event.Done.Invoke)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test207State.S01)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test207State) {
        when (state) {
            is Test207State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test207State,
        event: Test207Event?
    ) {
        when (source) {
        is Test207State.S01 -> when {
            event is Test207Event.ChildToParent -> {
            cancelSend("foo")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
