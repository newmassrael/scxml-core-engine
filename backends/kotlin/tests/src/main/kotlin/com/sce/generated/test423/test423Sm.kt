// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/423/test423.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test423.scxml:4 :: _machine

package com.sce.generated.test423

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test423State : State {
    data object Fail : Test423State
    data object Pass : Test423State
    data object S0 : Test423State
    data object S1 : Test423State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test423Event : Event {
    sealed interface Error : Test423Event {
        data object Execution : Error
    }
    data object ExternalEvent1 : Test423Event
    data object ExternalEvent2 : Test423Event
    data object InternalEvent : Test423Event
}
// --- State Machine (W3C SCXML) ---

class Test423StateMachine(
) : StateMachineEngine<Test423State, Test423Event>() {

    override val initialState: Test423State = Test423State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test423State? = when (stateId) {
        "fail" -> Test423State.Fail
        "pass" -> Test423State.Pass
        "s0" -> Test423State.S0
        "s1" -> Test423State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test423State): String = when (state) {
        is Test423State.Fail -> "fail"
        is Test423State.Pass -> "pass"
        is Test423State.S0 -> "s0"
        is Test423State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test423State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test423State): Int = when (state) {
        is Test423State.Fail -> 3
        is Test423State.Pass -> 2
        is Test423State.S0 -> 0
        is Test423State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test423State,
        event: Test423Event
    ): TransitionResult<Test423State> = when (state) {
        is Test423State.S0 -> processS0(event)
        is Test423State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test423Event
    ): TransitionResult<Test423State> = when {
        event is Test423Event.InternalEvent -> TransitionResult.External(Test423State.S1, Test423State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test423State.Fail, Test423State.S0)
    }

    private fun processS1(
        event: Test423Event
    ): TransitionResult<Test423State> = when {
        event is Test423Event.ExternalEvent2 -> TransitionResult.External(Test423State.Pass, Test423State.S1)

        event is Test423Event.InternalEvent -> TransitionResult.External(Test423State.Fail, Test423State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun onEntry(state: Test423State, pathChild: Test423State?) {
        when (state) {
            is Test423State.Fail -> {
                // SCE-MAP: test423.scxml:26 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.Pass -> {
                // SCE-MAP: test423.scxml:25 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.S0 -> {
                // SCE-MAP: test423.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test423Event.ExternalEvent1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            scheduleSend("__send_1", 1000L, Test423Event.ExternalEvent2)

            raiseInternal(Test423Event.InternalEvent)
            }
            is Test423State.S1 -> {
                // SCE-MAP: test423.scxml:18 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun onExit(state: Test423State) {
        when (state) {
            is Test423State.Fail -> {
                // SCE-MAP: test423.scxml:26 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test423State.Pass -> {
                // SCE-MAP: test423.scxml:25 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test423State.S0 -> {
                // SCE-MAP: test423.scxml:7 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test423State.S1 -> {
                // SCE-MAP: test423.scxml:18 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun executeTransitionActions(
        source: Test423State,
        event: Test423Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
