// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 97155411577c11a793c509f8eca92ed763090b054ea00a1be8ebdfe84ee878d0
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/237/test237.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test237.scxml:8

package com.sce.generated.test237

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test237State : State {
    data object Fail : Test237State
    data object Pass : Test237State
    data object S0 : Test237State
    data object S1 : Test237State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test237Event : Event {
    sealed interface Cancel : Test237Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test237Event {
        data object Invoke : Done
    }
    sealed interface Error : Test237Event {
        data object Execution : Error
    }
    data object Timeout1 : Test237Event
    data object Timeout2 : Test237Event
}
// --- State Machine (W3C SCXML) ---

class Test237StateMachine(
) : StateMachineEngine<Test237State, Test237Event>() {

    override val initialState: Test237State = Test237State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test237State? = when (stateId) {
        "fail" -> Test237State.Fail
        "pass" -> Test237State.Pass
        "s0" -> Test237State.S0
        "s1" -> Test237State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test237State): String = when (state) {
        is Test237State.Fail -> "fail"
        is Test237State.Pass -> "pass"
        is Test237State.S0 -> "s0"
        is Test237State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test237State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test237State): Int = when (state) {
        is Test237State.Fail -> 3
        is Test237State.Pass -> 2
        is Test237State.S0 -> 0
        is Test237State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test237Event? = when (name) {
        "cancel.invoke" -> Test237Event.Cancel.Invoke
        "done.invoke" -> Test237Event.Done.Invoke
        "error.execution" -> Test237Event.Error.Execution
        "timeout1" -> Test237Event.Timeout1
        "timeout2" -> Test237Event.Timeout2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test237Event): String? = when (event) {
        is Test237Event.Cancel.Invoke -> "cancel.invoke"
        is Test237Event.Done.Invoke -> "done.invoke"
        is Test237Event.Error.Execution -> "error.execution"
        is Test237Event.Timeout1 -> "timeout1"
        is Test237Event.Timeout2 -> "timeout2"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test237State,
        event: Test237Event
    ): TransitionResult<Test237State> = when (state) {
        is Test237State.S0 -> processS0(event)
        is Test237State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test237Event
    ): TransitionResult<Test237State> = when {
        event is Test237Event.Timeout1 -> TransitionResult.External(Test237State.S1, Test237State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test237Event
    ): TransitionResult<Test237State> = when {
        event is Test237Event.Done.Invoke -> TransitionResult.External(Test237State.Fail, Test237State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test237State.Pass, Test237State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test237.scxml:8
    override fun onEntry(state: Test237State) {
        when (state) {
            is Test237State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test237State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test237State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test237Event.Timeout1)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test237SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test237Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test237State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_1", 1500L, Test237Event.Timeout2)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test237.scxml:8
    override fun onExit(state: Test237State) {
        when (state) {
            is Test237State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test237State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test237State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test237State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test237.scxml:8
    override fun executeTransitionActions(
        source: Test237State,
        event: Test237Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
