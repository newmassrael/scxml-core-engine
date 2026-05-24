// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481

// GENERATED CODE — DO NOT EDIT
// Source: resources/509/test509.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test509.scxml:6

package com.sce.generated.test509

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test509State : State {
    data object Fail : Test509State
    data object Pass : Test509State
    data object S0 : Test509State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test509Event : Event {
    sealed interface Error : Test509Event {
        data object Execution : Error
    }
    data object Test : Test509Event
    data object Timeout : Test509Event
}
// --- State Machine (W3C SCXML) ---

class Test509StateMachine(
) : StateMachineEngine<Test509State, Test509Event>() {

    override val initialState: Test509State = Test509State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test509State? = when (stateId) {
        "fail" -> Test509State.Fail
        "pass" -> Test509State.Pass
        "s0" -> Test509State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test509State): String = when (state) {
        is Test509State.Fail -> "fail"
        is Test509State.Pass -> "pass"
        is Test509State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test509State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test509State): Int = when (state) {
        is Test509State.Fail -> 2
        is Test509State.Pass -> 1
        is Test509State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test509Event? = when (name) {
        "error.execution" -> Test509Event.Error.Execution
        "test" -> Test509Event.Test
        "timeout" -> Test509Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test509Event): String? = when (event) {
        is Test509Event.Error.Execution -> "error.execution"
        is Test509Event.Test -> "test"
        is Test509Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test509State,
        event: Test509Event
    ): TransitionResult<Test509State> = when (state) {
        is Test509State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test509Event
    ): TransitionResult<Test509State> = when {
        event is Test509Event.Test -> TransitionResult.External(Test509State.Pass, Test509State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test509State.Fail, Test509State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test509.scxml:6
    override fun onEntry(state: Test509State) {
        when (state) {
            is Test509State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test509State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test509State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 30000L, Test509Event.Timeout)



            performHttpSend("http://localhost:8080/test", "test", "", emptyMap(), "__send_1")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test509.scxml:6
    override fun onExit(state: Test509State) {
        when (state) {
            is Test509State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test509State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test509State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test509.scxml:6
    override fun executeTransitionActions(
        source: Test509State,
        event: Test509Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
