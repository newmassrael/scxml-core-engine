// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 39577af8fb5f7abbc502d5ae36e83f91b2556873f8c059eec3dff07c68aec183
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/348/test348.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test348.scxml:3

package com.sce.generated.test348

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test348State : State {
    data object Fail : Test348State
    data object Pass : Test348State
    data object S0 : Test348State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test348Event : Event {
    sealed interface Error : Test348Event {
        data object Execution : Error
    }
    data object S0Event : Test348Event
}
// --- State Machine (W3C SCXML) ---

class Test348StateMachine(
) : StateMachineEngine<Test348State, Test348Event>() {

    override val initialState: Test348State = Test348State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test348State? = when (stateId) {
        "fail" -> Test348State.Fail
        "pass" -> Test348State.Pass
        "s0" -> Test348State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test348State): String = when (state) {
        is Test348State.Fail -> "fail"
        is Test348State.Pass -> "pass"
        is Test348State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test348State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test348State): Int = when (state) {
        is Test348State.Fail -> 2
        is Test348State.Pass -> 1
        is Test348State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test348State,
        event: Test348Event
    ): TransitionResult<Test348State> = when (state) {
        is Test348State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test348Event
    ): TransitionResult<Test348State> = when {
        event is Test348Event.S0Event -> TransitionResult.External(Test348State.Pass, Test348State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test348State.Fail, Test348State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test348.scxml:3
    override fun onEntry(state: Test348State) {
        when (state) {
            is Test348State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test348State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test348State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test348Event.S0Event, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test348.scxml:3
    override fun onExit(state: Test348State) {
        when (state) {
            is Test348State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test348State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test348State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test348.scxml:3
    override fun executeTransitionActions(
        source: Test348State,
        event: Test348Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
