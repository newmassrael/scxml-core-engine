// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/200/test200.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test200.scxml:5

package com.sce.generated.test200

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test200State : State {
    data object Fail : Test200State
    data object Pass : Test200State
    data object S0 : Test200State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test200Event : Event {
    sealed interface Error : Test200Event {
        data object Execution : Error
    }
    data object Event1 : Test200Event
    data object Timeout : Test200Event
}
// --- State Machine (W3C SCXML) ---

class Test200StateMachine(
) : StateMachineEngine<Test200State, Test200Event>() {

    override val initialState: Test200State = Test200State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test200State? = when (stateId) {
        "fail" -> Test200State.Fail
        "pass" -> Test200State.Pass
        "s0" -> Test200State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test200State): String = when (state) {
        is Test200State.Fail -> "fail"
        is Test200State.Pass -> "pass"
        is Test200State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test200State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test200State): Int = when (state) {
        is Test200State.Fail -> 2
        is Test200State.Pass -> 1
        is Test200State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test200State,
        event: Test200Event
    ): TransitionResult<Test200State> = when (state) {
        is Test200State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test200Event
    ): TransitionResult<Test200State> = when {
        event is Test200Event.Event1 -> TransitionResult.External(Test200State.Pass, Test200State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test200State.Fail, Test200State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test200.scxml:5
    override fun onEntry(state: Test200State) {
        when (state) {
            is Test200State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test200State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test200State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test200Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            send(Test200Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test200.scxml:5
    override fun onExit(state: Test200State) {
        when (state) {
            is Test200State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test200State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test200State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test200.scxml:5
    override fun executeTransitionActions(
        source: Test200State,
        event: Test200Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
