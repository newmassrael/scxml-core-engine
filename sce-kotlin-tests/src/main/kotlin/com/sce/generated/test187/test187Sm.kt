// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912

// GENERATED CODE — DO NOT EDIT
// Source: resources/187/test187.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test187.scxml:7

package com.sce.generated.test187

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test187State : State {
    data object Fail : Test187State
    data object Pass : Test187State
    data object S0 : Test187State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test187Event : Event {
    sealed interface Cancel : Test187Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test187Event
    sealed interface Done : Test187Event {
        data object Invoke : Done
    }
    sealed interface Error : Test187Event {
        data object Execution : Error
    }
    data object Timeout : Test187Event
}
// --- State Machine (W3C SCXML) ---

class Test187StateMachine(
) : StateMachineEngine<Test187State, Test187Event>() {

    override val initialState: Test187State = Test187State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test187State? = when (stateId) {
        "fail" -> Test187State.Fail
        "pass" -> Test187State.Pass
        "s0" -> Test187State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test187State): String = when (state) {
        is Test187State.Fail -> "fail"
        is Test187State.Pass -> "pass"
        is Test187State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test187State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test187State): Int = when (state) {
        is Test187State.Fail -> 2
        is Test187State.Pass -> 1
        is Test187State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test187Event? = when (name) {
        "cancel.invoke" -> Test187Event.Cancel.Invoke
        "childToParent" -> Test187Event.ChildToParent
        "done.invoke" -> Test187Event.Done.Invoke
        "error.execution" -> Test187Event.Error.Execution
        "timeout" -> Test187Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test187Event): String? = when (event) {
        is Test187Event.Cancel.Invoke -> "cancel.invoke"
        is Test187Event.ChildToParent -> "childToParent"
        is Test187Event.Done.Invoke -> "done.invoke"
        is Test187Event.Error.Execution -> "error.execution"
        is Test187Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test187State,
        event: Test187Event
    ): TransitionResult<Test187State> = when (state) {
        is Test187State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test187Event
    ): TransitionResult<Test187State> = when {
        event is Test187Event.ChildToParent -> TransitionResult.External(Test187State.Fail, Test187State.S0)

        event is Test187Event.Timeout -> TransitionResult.External(Test187State.Pass, Test187State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test187.scxml:7
    override fun onEntry(state: Test187State) {
        when (state) {
            is Test187State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test187State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test187State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test187Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test187SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test187Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test187.scxml:7
    override fun onExit(state: Test187State) {
        when (state) {
            is Test187State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test187State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test187State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test187.scxml:7
    override fun executeTransitionActions(
        source: Test187State,
        event: Test187Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
