// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b5bef7d045160440c6e2790d4f2e0be757d7c1cc42dee75b2002b23fd477161e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/330/test330.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test330.scxml:5 :: _machine

package com.sce.generated.test330

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test330State : State {
    data object Fail : Test330State
    data object Pass : Test330State
    data object S0 : Test330State
    data object S1 : Test330State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test330Event : Event {
    sealed interface Error : Test330Event {
        data object Execution : Error
    }
    data object Foo : Test330Event
}
// --- State Machine (W3C SCXML) ---

class Test330StateMachine(
) : StateMachineEngine<Test330State, Test330Event>() {

    override val initialState: Test330State = Test330State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test330State? = when (stateId) {
        "fail" -> Test330State.Fail
        "pass" -> Test330State.Pass
        "s0" -> Test330State.S0
        "s1" -> Test330State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test330State): String = when (state) {
        is Test330State.Fail -> "fail"
        is Test330State.Pass -> "pass"
        is Test330State.S0 -> "s0"
        is Test330State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test330State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test330State): Int = when (state) {
        is Test330State.Fail -> 3
        is Test330State.Pass -> 2
        is Test330State.S0 -> 0
        is Test330State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test330State,
        event: Test330Event
    ): TransitionResult<Test330State> = when (state) {
        is Test330State.S0 -> processS0(event)
        is Test330State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test330Event
    ): TransitionResult<Test330State> = when {
        event is Test330Event.Foo -> TransitionResult.External(Test330State.S1, Test330State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test330State.Fail, Test330State.S0)
    }

    private fun processS1(
        event: Test330Event
    ): TransitionResult<Test330State> = when {
        event is Test330Event.Foo -> TransitionResult.External(Test330State.Pass, Test330State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test330State.Fail, Test330State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun onEntry(state: Test330State, pathChild: Test330State?) {
        when (state) {
            is Test330State.Fail -> {
                // SCE-MAP: test330.scxml:25 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.Pass -> {
                // SCE-MAP: test330.scxml:24 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.S0 -> {
                // SCE-MAP: test330.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test330Event.Foo)
            }
            is Test330State.S1 -> {
                // SCE-MAP: test330.scxml:15 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            send(Test330Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun onExit(state: Test330State) {
        when (state) {
            is Test330State.Fail -> {
                // SCE-MAP: test330.scxml:25 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test330State.Pass -> {
                // SCE-MAP: test330.scxml:24 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test330State.S0 -> {
                // SCE-MAP: test330.scxml:7 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test330State.S1 -> {
                // SCE-MAP: test330.scxml:15 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test330State,
        event: Test330Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
