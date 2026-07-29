// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c22d767976ad0f3af27597215acac4daa969b18394744727f9f1e4af8f5db2d7
// generated-at: 1785338317

// GENERATED CODE — DO NOT EDIT
// Source: resources/355/test355.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test355.scxml:5

package com.sce.generated.test355

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test355State : State {
    data object Fail : Test355State
    data object Pass : Test355State
    data object S0 : Test355State
    data object S1 : Test355State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test355Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test355StateMachine(
) : StateMachineEngine<Test355State, Test355Event>() {

    override val initialState: Test355State = Test355State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test355State? = when (stateId) {
        "fail" -> Test355State.Fail
        "pass" -> Test355State.Pass
        "s0" -> Test355State.S0
        "s1" -> Test355State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test355State): String = when (state) {
        is Test355State.Fail -> "fail"
        is Test355State.Pass -> "pass"
        is Test355State.S0 -> "s0"
        is Test355State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test355State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test355State): Int = when (state) {
        is Test355State.Fail -> 3
        is Test355State.Pass -> 2
        is Test355State.S0 -> 0
        is Test355State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test355State,
        event: Test355Event
    ): TransitionResult<Test355State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test355State
    ): TransitionResult<Test355State> = when (state) {
        is Test355State.S0 -> processNullS0()
        is Test355State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test355State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test355State.Pass, Test355State.S0)
    }

    private fun processNullS1(
    ): TransitionResult<Test355State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test355State.Fail, Test355State.S1)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test355.scxml:5
    override fun onEntry(state: Test355State) {
        when (state) {
            is Test355State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test355State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test355State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test355State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test355.scxml:5
    override fun onExit(state: Test355State) {
        when (state) {
            is Test355State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test355State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test355State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test355State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test355.scxml:5
    override fun executeTransitionActions(
        source: Test355State,
        event: Test355Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
