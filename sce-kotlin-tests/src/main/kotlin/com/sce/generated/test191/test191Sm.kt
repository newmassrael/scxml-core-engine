// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980218

// GENERATED CODE — DO NOT EDIT
// Source: resources/191/test191.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test191.scxml:6

package com.sce.generated.test191

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test191State : State {
    data object Fail : Test191State
    data object Pass : Test191State
    data object S0 : Test191State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test191Event : Event {
    sealed interface Cancel : Test191Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test191Event
    sealed interface Done : Test191Event {
        data object Invoke : Done
    }
    sealed interface Error : Test191Event {
        data object Execution : Error
    }
    data object Timeout : Test191Event
}
// --- State Machine (W3C SCXML) ---

class Test191StateMachine(
) : StateMachineEngine<Test191State, Test191Event>() {

    override val initialState: Test191State = Test191State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test191State? = when (stateId) {
        "fail" -> Test191State.Fail
        "pass" -> Test191State.Pass
        "s0" -> Test191State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test191State): String = when (state) {
        is Test191State.Fail -> "fail"
        is Test191State.Pass -> "pass"
        is Test191State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test191State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test191State): Int = when (state) {
        is Test191State.Fail -> 2
        is Test191State.Pass -> 1
        is Test191State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test191Event? = when (name) {
        "cancel.invoke" -> Test191Event.Cancel.Invoke
        "childToParent" -> Test191Event.ChildToParent
        "done.invoke" -> Test191Event.Done.Invoke
        "error.execution" -> Test191Event.Error.Execution
        "timeout" -> Test191Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test191Event): String? = when (event) {
        is Test191Event.Cancel.Invoke -> "cancel.invoke"
        is Test191Event.ChildToParent -> "childToParent"
        is Test191Event.Done.Invoke -> "done.invoke"
        is Test191Event.Error.Execution -> "error.execution"
        is Test191Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test191State,
        event: Test191Event
    ): TransitionResult<Test191State> = when (state) {
        is Test191State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test191Event
    ): TransitionResult<Test191State> = when {
        event is Test191Event.ChildToParent -> TransitionResult.External(Test191State.Pass, Test191State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test191State.Fail, Test191State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test191.scxml:6
    override fun onEntry(state: Test191State) {
        when (state) {
            is Test191State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test191State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test191State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test191Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test191SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test191Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test191.scxml:6
    override fun onExit(state: Test191State) {
        when (state) {
            is Test191State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test191State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test191State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test191.scxml:6
    override fun executeTransitionActions(
        source: Test191State,
        event: Test191Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
