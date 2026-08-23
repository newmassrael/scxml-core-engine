// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1f4fc251a4bb4df71320b116cc055aa1687156c3a3402c346abf1bd3694d0437
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/449/test449.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test449.scxml:5 :: _machine

package com.sce.generated.test449

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test449State : State {
    data object Fail : Test449State
    data object Pass : Test449State
    data object S0 : Test449State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test449Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test449StateMachine(
) : StateMachineEngine<Test449State, Test449Event>() {

    override val initialState: Test449State = Test449State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test449State? = when (stateId) {
        "fail" -> Test449State.Fail
        "pass" -> Test449State.Pass
        "s0" -> Test449State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test449State): String = when (state) {
        is Test449State.Fail -> "fail"
        is Test449State.Pass -> "pass"
        is Test449State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test449State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test449State): Int = when (state) {
        is Test449State.Fail -> 2
        is Test449State.Pass -> 1
        is Test449State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test449State,
        event: Test449Event
    ): TransitionResult<Test449State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test449State
    ): TransitionResult<Test449State> = when (state) {
        is Test449State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test449State> = when {
        true -> TransitionResult.External(Test449State.Pass, Test449State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test449State.Fail, Test449State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun onEntry(state: Test449State, pathChild: Test449State?) {
        when (state) {
            is Test449State.Fail -> {
                // SCE-MAP: test449.scxml:14 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test449State.Pass -> {
                // SCE-MAP: test449.scxml:13 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test449State.S0 -> {
                // SCE-MAP: test449.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun onExit(state: Test449State) {
        when (state) {
            is Test449State.Fail -> {
                // SCE-MAP: test449.scxml:14 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test449State.Pass -> {
                // SCE-MAP: test449.scxml:13 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test449State.S0 -> {
                // SCE-MAP: test449.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test449State,
        event: Test449Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
