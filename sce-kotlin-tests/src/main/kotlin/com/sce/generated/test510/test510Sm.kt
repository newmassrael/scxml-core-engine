// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425

// GENERATED CODE — DO NOT EDIT
// Source: resources/510/test510.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test510.scxml:5

package com.sce.generated.test510

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test510State : State {
    data object Fail : Test510State
    data object Pass : Test510State
    data object S0 : Test510State
    data object S1 : Test510State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test510Event : Event {
    sealed interface Error : Test510Event {
        data object Execution : Error
    }
    data object Internal : Test510Event
    data object Test : Test510Event
    data object Timeout : Test510Event
}
// --- State Machine (W3C SCXML) ---

class Test510StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test510State, Test510Event>(scriptEngine) {

    override val initialState: Test510State = Test510State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test510State? = when (stateId) {
        "fail" -> Test510State.Fail
        "pass" -> Test510State.Pass
        "s0" -> Test510State.S0
        "s1" -> Test510State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test510State): String = when (state) {
        is Test510State.Fail -> "fail"
        is Test510State.Pass -> "pass"
        is Test510State.S0 -> "s0"
        is Test510State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test510State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test510State): Int = when (state) {
        is Test510State.Fail -> 3
        is Test510State.Pass -> 2
        is Test510State.S0 -> 0
        is Test510State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test510Event? = when (name) {
        "error.execution" -> Test510Event.Error.Execution
        "internal" -> Test510Event.Internal
        "test" -> Test510Event.Test
        "timeout" -> Test510Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test510Event): String? = when (event) {
        is Test510Event.Error.Execution -> "error.execution"
        is Test510Event.Internal -> "internal"
        is Test510Event.Test -> "test"
        is Test510Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test510State,
        event: Test510Event
    ): TransitionResult<Test510State> = when (state) {
        is Test510State.S0 -> processS0(event)
        is Test510State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test510Event
    ): TransitionResult<Test510State> = when {
        event is Test510Event.Internal -> TransitionResult.External(Test510State.S1, Test510State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test510State.Fail, Test510State.S0)
    }

    private fun processS1(
        event: Test510Event
    ): TransitionResult<Test510State> = when {
        event is Test510Event.Test -> TransitionResult.External(Test510State.Pass, Test510State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test510State.Fail, Test510State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test510.scxml:5
    override fun onEntry(state: Test510State) {
        when (state) {
            is Test510State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test510State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test510State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 30000L, Test510Event.Timeout)



            performHttpSend("http://localhost:8080/test", "test", "", emptyMap(), "__send_1")

            raiseInternal(Test510Event.Internal)
            }
            is Test510State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test510.scxml:5
    override fun onExit(state: Test510State) {
        when (state) {
            is Test510State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test510State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test510State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test510State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test510.scxml:5
    override fun executeTransitionActions(
        source: Test510State,
        event: Test510Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
