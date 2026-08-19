// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 60da764009afb96185d876c542254f2e8363dba627394829757a2a8f121eddd1
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/236/test236.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test236.scxml:7 :: _machine

package com.sce.generated.test236

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test236State : State {
    data object Fail : Test236State
    data object Pass : Test236State
    data object S0 : Test236State
    data object S1 : Test236State
    data object S2 : Test236State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test236Event : Event {
    sealed interface Cancel : Test236Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test236Event
    sealed interface Done : Test236Event {
        data object Invoke : Done
    }
    sealed interface Error : Test236Event {
        data object Execution : Error
    }
    data object Timeout : Test236Event
}
// --- State Machine (W3C SCXML) ---

class Test236StateMachine(
) : StateMachineEngine<Test236State, Test236Event>() {

    override val initialState: Test236State = Test236State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test236State? = when (stateId) {
        "fail" -> Test236State.Fail
        "pass" -> Test236State.Pass
        "s0" -> Test236State.S0
        "s1" -> Test236State.S1
        "s2" -> Test236State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test236State): String = when (state) {
        is Test236State.Fail -> "fail"
        is Test236State.Pass -> "pass"
        is Test236State.S0 -> "s0"
        is Test236State.S1 -> "s1"
        is Test236State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test236State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test236State): Int = when (state) {
        is Test236State.Fail -> 4
        is Test236State.Pass -> 3
        is Test236State.S0 -> 0
        is Test236State.S1 -> 1
        is Test236State.S2 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test236Event? = when (name) {
        "cancel.invoke" -> Test236Event.Cancel.Invoke
        "childToParent" -> Test236Event.ChildToParent
        "done.invoke" -> Test236Event.Done.Invoke
        "error.execution" -> Test236Event.Error.Execution
        "timeout" -> Test236Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test236Event): String? = when (event) {
        is Test236Event.Cancel.Invoke -> "cancel.invoke"
        is Test236Event.ChildToParent -> "childToParent"
        is Test236Event.Done.Invoke -> "done.invoke"
        is Test236Event.Error.Execution -> "error.execution"
        is Test236Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test236State,
        event: Test236Event
    ): TransitionResult<Test236State> = when (state) {
        is Test236State.S0 -> processS0(event)
        is Test236State.S1 -> processS1(event)
        is Test236State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.ChildToParent -> TransitionResult.External(Test236State.S1, Test236State.S0)

        event is Test236Event.Done.Invoke -> TransitionResult.External(Test236State.Fail, Test236State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.Done.Invoke -> TransitionResult.External(Test236State.S2, Test236State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test236State.Fail, Test236State.S1)
    }

    private fun processS2(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.Timeout -> TransitionResult.External(Test236State.Pass, Test236State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test236State.Fail, Test236State.S2)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test236.scxml:7 :: _machine
    override fun onEntry(state: Test236State, pathChild: Test236State?) {
        when (state) {
            is Test236State.Fail -> {
                // SCE-MAP: test236.scxml:42 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test236State.Pass -> {
                // SCE-MAP: test236.scxml:41 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test236State.S0 -> {
                // SCE-MAP: test236.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test236Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test236SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test236Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test236State.S1 -> {
                // SCE-MAP: test236.scxml:30 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test236State.S2 -> {
                // SCE-MAP: test236.scxml:36 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test236.scxml:7 :: _machine
    override fun onExit(state: Test236State) {
        when (state) {
            is Test236State.Fail -> {
                // SCE-MAP: test236.scxml:42 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test236State.Pass -> {
                // SCE-MAP: test236.scxml:41 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test236State.S0 -> {
                // SCE-MAP: test236.scxml:10 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test236State.S1 -> {
                // SCE-MAP: test236.scxml:30 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test236State.S2 -> {
                // SCE-MAP: test236.scxml:36 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test236.scxml:7 :: _machine
    override fun executeTransitionActions(
        source: Test236State,
        event: Test236Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
