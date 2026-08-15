// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e136547eba5b1b26d444df3b244f86733d75a97e370ef305f7a135f66e51e2c8
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/185/test185.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test185.scxml:5 :: _machine

package com.sce.generated.test185

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test185State : State {
    data object Fail : Test185State
    data object Pass : Test185State
    data object S0 : Test185State
    data object S1 : Test185State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test185Event : Event {
    sealed interface Error : Test185Event {
        data object Execution : Error
    }
    data object Event1 : Test185Event
    data object Event2 : Test185Event
}
// --- State Machine (W3C SCXML) ---

class Test185StateMachine(
) : StateMachineEngine<Test185State, Test185Event>() {

    override val initialState: Test185State = Test185State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test185State? = when (stateId) {
        "fail" -> Test185State.Fail
        "pass" -> Test185State.Pass
        "s0" -> Test185State.S0
        "s1" -> Test185State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test185State): String = when (state) {
        is Test185State.Fail -> "fail"
        is Test185State.Pass -> "pass"
        is Test185State.S0 -> "s0"
        is Test185State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test185State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test185State): Int = when (state) {
        is Test185State.Fail -> 3
        is Test185State.Pass -> 2
        is Test185State.S0 -> 0
        is Test185State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test185State,
        event: Test185Event
    ): TransitionResult<Test185State> = when (state) {
        is Test185State.S0 -> processS0(event)
        is Test185State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test185Event
    ): TransitionResult<Test185State> = when {
        event is Test185Event.Event1 -> TransitionResult.External(Test185State.S1, Test185State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test185State.Fail, Test185State.S0)
    }

    private fun processS1(
        event: Test185Event
    ): TransitionResult<Test185State> = when {
        event is Test185Event.Event2 -> TransitionResult.External(Test185State.Pass, Test185State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test185State.Fail, Test185State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test185.scxml:5 :: _machine
    override fun onEntry(state: Test185State) {
        when (state) {
            is Test185State.Fail -> {
                // SCE-MAP: test185.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test185State.Pass -> {
                // SCE-MAP: test185.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test185State.S0 -> {
                // SCE-MAP: test185.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test185Event.Event2)


            send(Test185Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            is Test185State.S1 -> {
                // SCE-MAP: test185.scxml:18 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test185.scxml:5 :: _machine
    override fun onExit(state: Test185State) {
        when (state) {
            is Test185State.Fail -> {
                // SCE-MAP: test185.scxml:24 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test185State.Pass -> {
                // SCE-MAP: test185.scxml:23 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test185State.S0 -> {
                // SCE-MAP: test185.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test185State.S1 -> {
                // SCE-MAP: test185.scxml:18 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test185.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test185State,
        event: Test185Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
