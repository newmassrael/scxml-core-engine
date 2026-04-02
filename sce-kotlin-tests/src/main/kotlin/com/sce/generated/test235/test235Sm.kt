// GENERATED CODE — DO NOT EDIT
// Source: resources/235/test235.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test235

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test235State : State {
    data object Fail : Test235State
    data object Pass : Test235State
    data object S0 : Test235State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test235Event : Event {
    sealed interface Cancel : Test235Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test235Event {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Foo : Invoke
        }
    }
    sealed interface Error : Test235Event {
        data object Execution : Error
    }
    data object Timeout : Test235Event
}
// --- State Machine (W3C SCXML) ---

class Test235StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test235State, Test235Event>(scriptEngine) {

    override val initialState: Test235State = Test235State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test235Event? = when (name) {
        "cancel.invoke" -> Test235Event.Cancel.Invoke
        "done.invoke" -> Test235Event.Done.Invoke.Self
        "done.invoke.foo" -> Test235Event.Done.Invoke.Foo
        "error.execution" -> Test235Event.Error.Execution
        "timeout" -> Test235Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test235Event): String? = when (event) {
        is Test235Event.Cancel.Invoke -> "cancel.invoke"
        is Test235Event.Done.Invoke.Self -> "done.invoke"
        is Test235Event.Done.Invoke.Foo -> "done.invoke.foo"
        is Test235Event.Error.Execution -> "error.execution"
        is Test235Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test235State,
        event: Test235Event
    ): TransitionResult<Test235State> = when (state) {
        is Test235State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test235Event
    ): TransitionResult<Test235State> = when {
        event is Test235Event.Done.Invoke.Foo -> TransitionResult.External(Test235State.Pass, Test235State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test235State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test235State) {
        when (state) {
            is Test235State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.S0 -> {
            scheduleSend("__send_0", 2000L, Test235Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.foo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test235Child0StateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("foo", childSM, false, Test235Event.Done.Invoke.Foo, "", generatedInvokeId)
                    }
                }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test235State) {
        when (state) {
            is Test235State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("foo")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test235State,
        event: Test235Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
