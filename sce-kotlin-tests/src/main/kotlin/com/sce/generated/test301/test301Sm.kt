// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318

// GENERATED CODE — DO NOT EDIT
// Source: resources/301/test301.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test301.scxml:6

package com.sce.generated.test301

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test301State : State {
    data object Fail : Test301State
    data object Pass : Test301State
    data object S0 : Test301State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test301Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test301StateMachine(
) : StateMachineEngine<Test301State, Test301Event>() {

    override val initialState: Test301State = Test301State.Pass



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test301State? = when (stateId) {
        "fail" -> Test301State.Fail
        "pass" -> Test301State.Pass
        "s0" -> Test301State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test301State): String = when (state) {
        is Test301State.Fail -> "fail"
        is Test301State.Pass -> "pass"
        is Test301State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test301State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test301State): Int = when (state) {
        is Test301State.Fail -> 2
        is Test301State.Pass -> 1
        is Test301State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test301State,
        event: Test301Event
    ): TransitionResult<Test301State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test301State
    ): TransitionResult<Test301State> = when (state) {
        is Test301State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test301State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test301State.Fail, Test301State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test301.scxml:6
    override fun onEntry(state: Test301State) {
        when (state) {
            is Test301State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test301State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test301State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test301.scxml:6
    override fun onExit(state: Test301State) {
        when (state) {
            is Test301State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test301State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test301State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test301.scxml:6
    override fun executeTransitionActions(
        source: Test301State,
        event: Test301Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
