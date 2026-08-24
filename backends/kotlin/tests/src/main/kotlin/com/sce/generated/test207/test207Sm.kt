// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6d29ccd65cc69c7036210e21d4c9d2a46b7717262dc7e045f86a45620f80383f
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/207/test207.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test207.scxml:8 :: _machine

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
) : StateMachineEngine<Test207State, Test207Event>() {

    override val initialState: Test207State = Test207State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

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

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test207State? = when (stateId) {
        "fail" -> Test207State.Fail
        "pass" -> Test207State.Pass
        "s0" -> Test207State.S0
        "s01" -> Test207State.S01
        "s02" -> Test207State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test207State): String = when (state) {
        is Test207State.Fail -> "fail"
        is Test207State.Pass -> "pass"
        is Test207State.S0 -> "s0"
        is Test207State.S01 -> "s01"
        is Test207State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test207State): Boolean = when (state) {
        is Test207State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test207State): Int = when (state) {
        is Test207State.Fail -> 4
        is Test207State.Pass -> 3
        is Test207State.S0 -> 0
        is Test207State.S01 -> 1
        is Test207State.S02 -> 2
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
        event is Test207Event.ChildToParent -> TransitionResult.External(Test207State.S02, Test207State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test207Event
    ): TransitionResult<Test207State> = when {
        event is Test207Event.Pass -> TransitionResult.External(Test207State.Pass, Test207State.S02)

        event is Test207Event.Fail -> TransitionResult.External(Test207State.Fail, Test207State.S02)

        event is Test207Event.Timeout -> TransitionResult.External(Test207State.Fail, Test207State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun onEntry(state: Test207State, pathChild: Test207State?) {
        when (state) {
            is Test207State.Fail -> {
                // SCE-MAP: test207.scxml:58 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test207State.Pass -> {
                // SCE-MAP: test207.scxml:57 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test207State.S0 -> {
                // SCE-MAP: test207.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test207Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test207SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test207Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test207State.S01 -> {
                // SCE-MAP: test207.scxml:43 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test207State.S02 -> {
                // SCE-MAP: test207.scxml:49 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun onExit(state: Test207State) {
        when (state) {
            is Test207State.Fail -> {
                // SCE-MAP: test207.scxml:58 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test207State.Pass -> {
                // SCE-MAP: test207.scxml:57 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test207State.S0 -> {
                // SCE-MAP: test207.scxml:11 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test207State.S01 -> {
                // SCE-MAP: test207.scxml:43 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test207State.S02 -> {
                // SCE-MAP: test207.scxml:49 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun executeTransitionActions(
        source: Test207State,
        event: Test207Event?
    ) {
        when (source) {
        is Test207State.S01 -> when {
            event is Test207Event.ChildToParent -> {
                // SCE-MAP: test207.scxml:44 :: s01 :: _transition_0


            cancelSend("foo")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
