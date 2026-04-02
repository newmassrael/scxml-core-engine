// GENERATED CODE — DO NOT EDIT
// Source: resources/232/test232.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test232

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test232State : State {
    data object Fail : Test232State
    data object Pass : Test232State
    data object S0 : Test232State
    data object S01 : Test232State
    data object S02 : Test232State
    data object S03 : Test232State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232Event : Event {
    sealed interface Cancel : Test232Event {
        data object Invoke : Cancel
    }
    data object ChildToParent1 : Test232Event
    data object ChildToParent2 : Test232Event
    sealed interface Done : Test232Event {
        data object Invoke : Done
    }
    sealed interface Error : Test232Event {
        data object Execution : Error
    }
    data object Timeout : Test232Event
}
// --- State Machine (W3C SCXML) ---

class Test232StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test232State, Test232Event>(scriptEngine) {

    override val initialState: Test232State = Test232State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test232State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test232State): Test232State? = when (state) {
        is Test232State.S01 -> Test232State.S0
        is Test232State.S02 -> Test232State.S0
        is Test232State.S03 -> Test232State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test232State): Test232State = when (state) {
        is Test232State.S0 -> Test232State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232Event? = when (name) {
        "cancel.invoke" -> Test232Event.Cancel.Invoke
        "childToParent1" -> Test232Event.ChildToParent1
        "childToParent2" -> Test232Event.ChildToParent2
        "done.invoke" -> Test232Event.Done.Invoke
        "error.execution" -> Test232Event.Error.Execution
        "timeout" -> Test232Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232Event): String? = when (event) {
        is Test232Event.Cancel.Invoke -> "cancel.invoke"
        is Test232Event.ChildToParent1 -> "childToParent1"
        is Test232Event.ChildToParent2 -> "childToParent2"
        is Test232Event.Done.Invoke -> "done.invoke"
        is Test232Event.Error.Execution -> "error.execution"
        is Test232Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test232State,
        event: Test232Event
    ): TransitionResult<Test232State> = when (state) {
        is Test232State.S0 -> processS0(event)
        is Test232State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test232State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test232State.S03 -> {
            val result = processS03(event)
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
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.Timeout -> TransitionResult.External(Test232State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.ChildToParent1 -> TransitionResult.External(Test232State.S02)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.ChildToParent2 -> TransitionResult.External(Test232State.S03)
        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.Done.Invoke -> TransitionResult.External(Test232State.Pass)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test232State) {
        when (state) {
            is Test232State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.S0 -> {
            scheduleSend("__send_0", 3000L, Test232Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test232Child0StateMachine(scriptEngine), false, Test232Event.Done.Invoke)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test232State.S01)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test232State) {
        when (state) {
            is Test232State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test232State,
        event: Test232Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
