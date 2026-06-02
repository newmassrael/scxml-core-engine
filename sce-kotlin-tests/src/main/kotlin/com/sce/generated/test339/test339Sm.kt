// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263

// GENERATED CODE — DO NOT EDIT
// Source: resources/339/test339.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test339.scxml:5

package com.sce.generated.test339

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test339State : State {
    data object Fail : Test339State
    data object Pass : Test339State
    data object S0 : Test339State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test339Event : Event {
    data object Foo : Test339Event
}
// --- State Machine (W3C SCXML) ---

class Test339StateMachine(
) : StateMachineEngine<Test339State, Test339Event>() {

    override val initialState: Test339State = Test339State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test339State? = when (stateId) {
        "fail" -> Test339State.Fail
        "pass" -> Test339State.Pass
        "s0" -> Test339State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test339State): String = when (state) {
        is Test339State.Fail -> "fail"
        is Test339State.Pass -> "pass"
        is Test339State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test339State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test339State): Int = when (state) {
        is Test339State.Fail -> 2
        is Test339State.Pass -> 1
        is Test339State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test339State,
        event: Test339Event
    ): TransitionResult<Test339State> = when (state) {
        is Test339State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test339Event
    ): TransitionResult<Test339State> = when {
        event is Test339Event.Foo -> TransitionResult.External(Test339State.Pass, Test339State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test339State.Fail, Test339State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test339.scxml:5
    override fun onEntry(state: Test339State) {
        when (state) {
            is Test339State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test339State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test339State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test339Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test339.scxml:5
    override fun onExit(state: Test339State) {
        when (state) {
            is Test339State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test339State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test339State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test339.scxml:5
    override fun executeTransitionActions(
        source: Test339State,
        event: Test339Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
