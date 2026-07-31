// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/337/test337.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test337.scxml:5

package com.sce.generated.test337

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test337State : State {
    data object Fail : Test337State
    data object Pass : Test337State
    data object S0 : Test337State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test337Event : Event {
    data object Foo : Test337Event
}
// --- State Machine (W3C SCXML) ---

class Test337StateMachine(
) : StateMachineEngine<Test337State, Test337Event>() {

    override val initialState: Test337State = Test337State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test337State? = when (stateId) {
        "fail" -> Test337State.Fail
        "pass" -> Test337State.Pass
        "s0" -> Test337State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test337State): String = when (state) {
        is Test337State.Fail -> "fail"
        is Test337State.Pass -> "pass"
        is Test337State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test337State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test337State): Int = when (state) {
        is Test337State.Fail -> 2
        is Test337State.Pass -> 1
        is Test337State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test337State,
        event: Test337Event
    ): TransitionResult<Test337State> = when (state) {
        is Test337State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test337Event
    ): TransitionResult<Test337State> = when {
        event is Test337Event.Foo -> TransitionResult.External(Test337State.Pass, Test337State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test337State.Fail, Test337State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test337.scxml:5
    override fun onEntry(state: Test337State) {
        when (state) {
            is Test337State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test337State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test337State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test337Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test337.scxml:5
    override fun onExit(state: Test337State) {
        when (state) {
            is Test337State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test337State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test337State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test337.scxml:5
    override fun executeTransitionActions(
        source: Test337State,
        event: Test337Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
