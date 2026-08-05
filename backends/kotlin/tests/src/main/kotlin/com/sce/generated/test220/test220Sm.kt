// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7afc591fda192b42ad8a433570c001416f9be57edde17b6193960abf579021c2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/220/test220.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test220.scxml:5

package com.sce.generated.test220

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test220State : State {
    data object Fail : Test220State
    data object Pass : Test220State
    data object S0 : Test220State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test220Event : Event {
    sealed interface Cancel : Test220Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test220Event {
        data object Invoke : Done
    }
    sealed interface Error : Test220Event {
        data object Execution : Error
    }
    data object Timeout : Test220Event
}
// --- State Machine (W3C SCXML) ---

class Test220StateMachine(
) : StateMachineEngine<Test220State, Test220Event>() {

    override val initialState: Test220State = Test220State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test220State? = when (stateId) {
        "fail" -> Test220State.Fail
        "pass" -> Test220State.Pass
        "s0" -> Test220State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test220State): String = when (state) {
        is Test220State.Fail -> "fail"
        is Test220State.Pass -> "pass"
        is Test220State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test220State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test220State): Int = when (state) {
        is Test220State.Fail -> 2
        is Test220State.Pass -> 1
        is Test220State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test220Event? = when (name) {
        "cancel.invoke" -> Test220Event.Cancel.Invoke
        "done.invoke" -> Test220Event.Done.Invoke
        "error.execution" -> Test220Event.Error.Execution
        "timeout" -> Test220Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test220Event): String? = when (event) {
        is Test220Event.Cancel.Invoke -> "cancel.invoke"
        is Test220Event.Done.Invoke -> "done.invoke"
        is Test220Event.Error.Execution -> "error.execution"
        is Test220Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test220State,
        event: Test220Event
    ): TransitionResult<Test220State> = when (state) {
        is Test220State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test220Event
    ): TransitionResult<Test220State> = when {
        event is Test220Event.Done.Invoke -> TransitionResult.External(Test220State.Pass, Test220State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test220State.Fail, Test220State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test220.scxml:5
    override fun onEntry(state: Test220State) {
        when (state) {
            is Test220State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test220Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test220SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test220Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test220.scxml:5
    override fun onExit(state: Test220State) {
        when (state) {
            is Test220State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test220State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test220State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test220.scxml:5
    override fun executeTransitionActions(
        source: Test220State,
        event: Test220Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
