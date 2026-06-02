// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943

// GENERATED CODE — DO NOT EDIT
// Source: resources/201/test201.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test201.scxml:6

package com.sce.generated.test201

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test201State : State {
    data object Fail : Test201State
    data object Pass : Test201State
    data object S0 : Test201State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test201Event : Event {
    sealed interface Error : Test201Event {
        data object Execution : Error
    }
    data object Event1 : Test201Event
    data object Timeout : Test201Event
}
// --- State Machine (W3C SCXML) ---

class Test201StateMachine(
) : StateMachineEngine<Test201State, Test201Event>() {

    override val initialState: Test201State = Test201State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test201State? = when (stateId) {
        "fail" -> Test201State.Fail
        "pass" -> Test201State.Pass
        "s0" -> Test201State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test201State): String = when (state) {
        is Test201State.Fail -> "fail"
        is Test201State.Pass -> "pass"
        is Test201State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test201State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test201State): Int = when (state) {
        is Test201State.Fail -> 2
        is Test201State.Pass -> 1
        is Test201State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test201Event? = when (name) {
        "error.execution" -> Test201Event.Error.Execution
        "event1" -> Test201Event.Event1
        "timeout" -> Test201Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test201Event): String? = when (event) {
        is Test201Event.Error.Execution -> "error.execution"
        is Test201Event.Event1 -> "event1"
        is Test201Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test201State,
        event: Test201Event
    ): TransitionResult<Test201State> = when (state) {
        is Test201State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test201Event
    ): TransitionResult<Test201State> = when {
        event is Test201Event.Event1 -> TransitionResult.External(Test201State.Pass, Test201State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test201State.Fail, Test201State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test201.scxml:6
    override fun onEntry(state: Test201State) {
        when (state) {
            is Test201State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test201State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test201State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return



            performHttpSend("http://localhost:8080/test", "event1", "", emptyMap(), "__send_0")


            send(Test201Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test201.scxml:6
    override fun onExit(state: Test201State) {
        when (state) {
            is Test201State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test201State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test201State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test201.scxml:6
    override fun executeTransitionActions(
        source: Test201State,
        event: Test201Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
