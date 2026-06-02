// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263

// GENERATED CODE — DO NOT EDIT
// Source: resources/495/test495.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test495.scxml:4

package com.sce.generated.test495

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test495State : State {
    data object Fail : Test495State
    data object Pass : Test495State
    data object S0 : Test495State
    data object S1 : Test495State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test495Event : Event {
    sealed interface Error : Test495Event {
        data object Execution : Error
    }
    data object Event1 : Test495Event
    data object Event2 : Test495Event
}
// --- State Machine (W3C SCXML) ---

class Test495StateMachine(
) : StateMachineEngine<Test495State, Test495Event>() {

    override val initialState: Test495State = Test495State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test495State? = when (stateId) {
        "fail" -> Test495State.Fail
        "pass" -> Test495State.Pass
        "s0" -> Test495State.S0
        "s1" -> Test495State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test495State): String = when (state) {
        is Test495State.Fail -> "fail"
        is Test495State.Pass -> "pass"
        is Test495State.S0 -> "s0"
        is Test495State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test495State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test495State): Int = when (state) {
        is Test495State.Fail -> 3
        is Test495State.Pass -> 2
        is Test495State.S0 -> 0
        is Test495State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test495State,
        event: Test495Event
    ): TransitionResult<Test495State> = when (state) {
        is Test495State.S0 -> processS0(event)
        is Test495State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test495Event
    ): TransitionResult<Test495State> = when {
        event is Test495Event.Event1 -> TransitionResult.External(Test495State.Fail, Test495State.S0)

        event is Test495Event.Event2 -> TransitionResult.External(Test495State.S1, Test495State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test495Event
    ): TransitionResult<Test495State> = when {
        event is Test495Event.Event1 -> TransitionResult.External(Test495State.Pass, Test495State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test495State.Fail, Test495State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test495.scxml:4
    override fun onEntry(state: Test495State) {
        when (state) {
            is Test495State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test495State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test495State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test495Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            raiseInternal(Test495Event.Event2)
            }
            is Test495State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test495.scxml:4
    override fun onExit(state: Test495State) {
        when (state) {
            is Test495State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test495State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test495State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test495State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test495.scxml:4
    override fun executeTransitionActions(
        source: Test495State,
        event: Test495Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
