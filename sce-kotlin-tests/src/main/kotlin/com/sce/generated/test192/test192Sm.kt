// GENERATED CODE — DO NOT EDIT
// Source: resources/192/test192.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test192

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test192State : State {
    data object Fail : Test192State
    data object Pass : Test192State
    data object S0 : Test192State
    data object S01 : Test192State
    data object S02 : Test192State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test192Event : Event {
    sealed interface Cancel : Test192Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test192Event
    sealed interface Done : Test192Event {
        data object Invoke : Done
    }
    sealed interface Error : Test192Event {
        data object Execution : Error
    }
    data object EventReceived : Test192Event
    data object ParentToChild : Test192Event
    data object Timeout : Test192Event
}
// --- State Machine (W3C SCXML) ---

class Test192StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test192State, Test192Event>(scriptEngine) {

    override val initialState: Test192State = Test192State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test192State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test192State): Test192State? = when (state) {
        is Test192State.S01 -> Test192State.S0
        is Test192State.S02 -> Test192State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test192State): Test192State = when (state) {
        is Test192State.S0 -> Test192State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test192Event? = when (name) {
        "cancel.invoke" -> Test192Event.Cancel.Invoke
        "childToParent" -> Test192Event.ChildToParent
        "done.invoke" -> Test192Event.Done.Invoke
        "error.execution" -> Test192Event.Error.Execution
        "eventReceived" -> Test192Event.EventReceived
        "parentToChild" -> Test192Event.ParentToChild
        "timeout" -> Test192Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test192Event): String? = when (event) {
        is Test192Event.Cancel.Invoke -> "cancel.invoke"
        is Test192Event.ChildToParent -> "childToParent"
        is Test192Event.Done.Invoke -> "done.invoke"
        is Test192Event.Error.Execution -> "error.execution"
        is Test192Event.EventReceived -> "eventReceived"
        is Test192Event.ParentToChild -> "parentToChild"
        is Test192Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test192State,
        event: Test192Event
    ): TransitionResult<Test192State> = when (state) {
        is Test192State.S0 -> processS0(event)
        is Test192State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test192State.S02 -> {
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
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.Timeout -> TransitionResult.External(Test192State.Fail)
        event is Test192Event.Done.Invoke -> TransitionResult.External(Test192State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.ChildToParent -> TransitionResult.External(Test192State.S02)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.EventReceived -> TransitionResult.External(Test192State.Pass)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test192State) {
        when (state) {
            is Test192State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test192State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test192State.S0 -> {
            scheduleSend("__send_0", 5000L, Test192Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("invokedChild", Test192Child0StateMachine(scriptEngine), false, Test192Event.Done.Invoke)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test192State.S01)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test192State) {
        when (state) {
            is Test192State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("invokedChild")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test192State,
        event: Test192Event?
    ) {
        when (source) {
        is Test192State.S01 -> when {
            event is Test192Event.ChildToParent -> {
            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("invokedChild", "parentToChild")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
