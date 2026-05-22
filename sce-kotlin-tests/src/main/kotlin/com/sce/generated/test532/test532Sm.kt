// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271

// GENERATED CODE — DO NOT EDIT
// Source: resources/532/test532.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test532.scxml:4

package com.sce.generated.test532

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test532State : State {
    data object Fail : Test532State
    data object Pass : Test532State
    data object S0 : Test532State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test532Event : Event {
    data object Empty : Test532Event
    sealed interface HTTP : Test532Event {
        data object POST : HTTP
    }
    sealed interface Error : Test532Event {
        data object Execution : Error
    }
    data object Timeout : Test532Event
}
// --- State Machine (W3C SCXML) ---

class Test532StateMachine(
) : StateMachineEngine<Test532State, Test532Event>() {

    override val initialState: Test532State = Test532State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test532State? = when (stateId) {
        "fail" -> Test532State.Fail
        "pass" -> Test532State.Pass
        "s0" -> Test532State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test532State): String = when (state) {
        is Test532State.Fail -> "fail"
        is Test532State.Pass -> "pass"
        is Test532State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test532State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test532State): Int = when (state) {
        is Test532State.Fail -> 2
        is Test532State.Pass -> 1
        is Test532State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test532Event? = when (name) {
        "" -> Test532Event.Empty
        "error.execution" -> Test532Event.Error.Execution
        "HTTP.POST" -> Test532Event.HTTP.POST
        "timeout" -> Test532Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test532Event): String? = when (event) {
        is Test532Event.Empty -> ""
        is Test532Event.Error.Execution -> "error.execution"
        is Test532Event.HTTP.POST -> "HTTP.POST"
        is Test532Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test532State,
        event: Test532Event
    ): TransitionResult<Test532State> = when (state) {
        is Test532State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test532Event
    ): TransitionResult<Test532State> = when {
        event is Test532Event.HTTP.POST -> TransitionResult.External(Test532State.Pass, Test532State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test532State.Fail, Test532State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test532.scxml:4
    override fun onEntry(state: Test532State) {
        when (state) {
            is Test532State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test532Event.Timeout)



            performHttpSend("http://localhost:8080/test", "", "some content", emptyMap(), "__send_1")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test532.scxml:4
    override fun onExit(state: Test532State) {
        when (state) {
            is Test532State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test532State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test532State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test532.scxml:4
    override fun executeTransitionActions(
        source: Test532State,
        event: Test532Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
