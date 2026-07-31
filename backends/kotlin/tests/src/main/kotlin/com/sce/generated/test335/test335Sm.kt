// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/335/test335.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test335.scxml:5

package com.sce.generated.test335

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test335State : State {
    data object Fail : Test335State
    data object Pass : Test335State
    data object S0 : Test335State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test335Event : Event {
    data object Foo : Test335Event
}
// --- State Machine (W3C SCXML) ---

class Test335StateMachine(
) : StateMachineEngine<Test335State, Test335Event>() {

    override val initialState: Test335State = Test335State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test335State? = when (stateId) {
        "fail" -> Test335State.Fail
        "pass" -> Test335State.Pass
        "s0" -> Test335State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test335State): String = when (state) {
        is Test335State.Fail -> "fail"
        is Test335State.Pass -> "pass"
        is Test335State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test335State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test335State): Int = when (state) {
        is Test335State.Fail -> 2
        is Test335State.Pass -> 1
        is Test335State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test335State,
        event: Test335Event
    ): TransitionResult<Test335State> = when (state) {
        is Test335State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test335Event
    ): TransitionResult<Test335State> = when {
        event is Test335Event.Foo -> TransitionResult.External(Test335State.Pass, Test335State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test335State.Fail, Test335State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test335.scxml:5
    override fun onEntry(state: Test335State) {
        when (state) {
            is Test335State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test335State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test335State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test335Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test335.scxml:5
    override fun onExit(state: Test335State) {
        when (state) {
            is Test335State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test335State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test335State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test335.scxml:5
    override fun executeTransitionActions(
        source: Test335State,
        event: Test335Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
