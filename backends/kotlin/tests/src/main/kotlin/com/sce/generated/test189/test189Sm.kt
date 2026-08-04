// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 817d9c061804919d9748138703a11f334a156a4e2a1e5a3c66f1c4e7ca554aa2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/189/test189.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test189.scxml:6

package com.sce.generated.test189

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test189State : State {
    data object Fail : Test189State
    data object Pass : Test189State
    data object S0 : Test189State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test189Event : Event {
    sealed interface Error : Test189Event {
        data object Execution : Error
    }
    data object Event1 : Test189Event
    data object Event2 : Test189Event
}
// --- State Machine (W3C SCXML) ---

class Test189StateMachine(
) : StateMachineEngine<Test189State, Test189Event>() {

    override val initialState: Test189State = Test189State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test189State? = when (stateId) {
        "fail" -> Test189State.Fail
        "pass" -> Test189State.Pass
        "s0" -> Test189State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test189State): String = when (state) {
        is Test189State.Fail -> "fail"
        is Test189State.Pass -> "pass"
        is Test189State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test189State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test189State): Int = when (state) {
        is Test189State.Fail -> 2
        is Test189State.Pass -> 1
        is Test189State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test189State,
        event: Test189Event
    ): TransitionResult<Test189State> = when (state) {
        is Test189State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test189Event
    ): TransitionResult<Test189State> = when {
        event is Test189Event.Event1 -> TransitionResult.External(Test189State.Pass, Test189State.S0)

        event is Test189Event.Event2 -> TransitionResult.External(Test189State.Fail, Test189State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test189.scxml:6
    override fun onEntry(state: Test189State) {
        when (state) {
            is Test189State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test189Event.Event2, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            raiseInternal(Test189Event.Event1)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test189.scxml:6
    override fun onExit(state: Test189State) {
        when (state) {
            is Test189State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test189State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test189State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test189.scxml:6
    override fun executeTransitionActions(
        source: Test189State,
        event: Test189Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
