// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213

// GENERATED CODE — DO NOT EDIT
// Source: resources/199/test199.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test199.scxml:5

package com.sce.generated.test199

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test199State : State {
    data object Fail : Test199State
    data object Pass : Test199State
    data object S0 : Test199State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test199Event : Event {
    sealed interface Error : Test199Event {
        data object Execution : Error
    }
    data object Event1 : Test199Event
    data object Timeout : Test199Event
}
// --- State Machine (W3C SCXML) ---

class Test199StateMachine(
) : StateMachineEngine<Test199State, Test199Event>() {

    override val initialState: Test199State = Test199State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test199State? = when (stateId) {
        "fail" -> Test199State.Fail
        "pass" -> Test199State.Pass
        "s0" -> Test199State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test199State): String = when (state) {
        is Test199State.Fail -> "fail"
        is Test199State.Pass -> "pass"
        is Test199State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test199State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test199State): Int = when (state) {
        is Test199State.Fail -> 2
        is Test199State.Pass -> 1
        is Test199State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test199State,
        event: Test199Event
    ): TransitionResult<Test199State> = when (state) {
        is Test199State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test199Event
    ): TransitionResult<Test199State> = when {
        event is Test199Event.Error.Execution -> TransitionResult.External(Test199State.Pass, Test199State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test199State.Fail, Test199State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test199.scxml:5
    override fun onEntry(state: Test199State) {
        when (state) {
            is Test199State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raiseInternal(Test199Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content


            send(Test199Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test199.scxml:5
    override fun onExit(state: Test199State) {
        when (state) {
            is Test199State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test199State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test199State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test199.scxml:5
    override fun executeTransitionActions(
        source: Test199State,
        event: Test199Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
