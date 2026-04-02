// GENERATED CODE — DO NOT EDIT
// Source: resources/247/test247.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test247

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test247State : State {
    data object Fail : Test247State
    data object Pass : Test247State
    data object S0 : Test247State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test247Event : Event {
    sealed interface Cancel : Test247Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test247Event {
        data object Invoke : Done
    }
    sealed interface Error : Test247Event {
        data object Execution : Error
    }
    data object Timeout : Test247Event
}
// --- State Machine (W3C SCXML) ---

class Test247StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test247State, Test247Event>(scriptEngine) {

    override val initialState: Test247State = Test247State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test247Event? = when (name) {
        "cancel.invoke" -> Test247Event.Cancel.Invoke
        "done.invoke" -> Test247Event.Done.Invoke
        "error.execution" -> Test247Event.Error.Execution
        "timeout" -> Test247Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test247Event): String? = when (event) {
        is Test247Event.Cancel.Invoke -> "cancel.invoke"
        is Test247Event.Done.Invoke -> "done.invoke"
        is Test247Event.Error.Execution -> "error.execution"
        is Test247Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test247State,
        event: Test247Event
    ): TransitionResult<Test247State> = when (state) {
        is Test247State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test247Event
    ): TransitionResult<Test247State> = when {
        event is Test247Event.Done.Invoke -> TransitionResult.External(Test247State.Pass, Test247State.S0)

        event is Test247Event.Timeout -> TransitionResult.External(Test247State.Fail, Test247State.S0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test247State) {
        when (state) {
            is Test247State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test247State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test247State.S0 -> {
            scheduleSend("__send_0", 2000L, Test247Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test247Child0StateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test247Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test247State) {
        when (state) {
            is Test247State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test247State,
        event: Test247Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
