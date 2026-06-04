// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e03d007af0e666370768a5b0be76775e8be2eb913728a32c0bf7ae79d6929af0
// generated-at: 1780566007

// GENERATED CODE — DO NOT EDIT
// Source: resources/531/test531.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test531.scxml:4

package com.sce.generated.test531

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test531State : State {
    data object Fail : Test531State
    data object Pass : Test531State
    data object S0 : Test531State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test531Event : Event {
    sealed interface Error : Test531Event {
        data object Execution : Error
    }
    data object Test : Test531Event
    data object Timeout : Test531Event
}
// --- State Machine (W3C SCXML) ---

class Test531StateMachine(
) : StateMachineEngine<Test531State, Test531Event>() {

    override val initialState: Test531State = Test531State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test531State? = when (stateId) {
        "fail" -> Test531State.Fail
        "pass" -> Test531State.Pass
        "s0" -> Test531State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test531State): String = when (state) {
        is Test531State.Fail -> "fail"
        is Test531State.Pass -> "pass"
        is Test531State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test531State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test531State): Int = when (state) {
        is Test531State.Fail -> 2
        is Test531State.Pass -> 1
        is Test531State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test531Event? = when (name) {
        "error.execution" -> Test531Event.Error.Execution
        "test" -> Test531Event.Test
        "timeout" -> Test531Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test531Event): String? = when (event) {
        is Test531Event.Error.Execution -> "error.execution"
        is Test531Event.Test -> "test"
        is Test531Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test531State,
        event: Test531Event
    ): TransitionResult<Test531State> = when (state) {
        is Test531State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test531Event
    ): TransitionResult<Test531State> = when {
        event is Test531Event.Test -> TransitionResult.External(Test531State.Pass, Test531State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test531State.Fail, Test531State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test531.scxml:4
    override fun onEntry(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test531Event.Timeout)



            // W3C SCXML C.2: BasicHTTP send with static params
            run {
                val httpParams = mutableMapOf<String, List<String>>()
                httpParams["_scxmleventname"] = listOf("test")
                performHttpSend("http://localhost:8080/test", "", "", httpParams, "__send_1")
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test531.scxml:4
    override fun onExit(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test531State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test531State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test531.scxml:4
    override fun executeTransitionActions(
        source: Test531State,
        event: Test531Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
