// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: dbfa9cca1428438cf4178bb8fcf463f9b9d0c7c649f4bf0e0f3de90abcfd2a47
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/333/test333.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test333.scxml:5

package com.sce.generated.test333

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test333State : State {
    data object Fail : Test333State
    data object Pass : Test333State
    data object S0 : Test333State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test333Event : Event {
    sealed interface Error : Test333Event {
        data object Execution : Error
    }
    data object Foo : Test333Event
}
// --- State Machine (W3C SCXML) ---

class Test333StateMachine(
) : StateMachineEngine<Test333State, Test333Event>() {

    override val initialState: Test333State = Test333State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test333State? = when (stateId) {
        "fail" -> Test333State.Fail
        "pass" -> Test333State.Pass
        "s0" -> Test333State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test333State): String = when (state) {
        is Test333State.Fail -> "fail"
        is Test333State.Pass -> "pass"
        is Test333State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test333State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test333State): Int = when (state) {
        is Test333State.Fail -> 2
        is Test333State.Pass -> 1
        is Test333State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test333State,
        event: Test333Event
    ): TransitionResult<Test333State> = when (state) {
        is Test333State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test333Event
    ): TransitionResult<Test333State> = when {
        event is Test333Event.Foo -> TransitionResult.External(Test333State.Pass, Test333State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test333State.Fail, Test333State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test333.scxml:5
    override fun onEntry(state: Test333State) {
        when (state) {
            is Test333State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test333Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test333.scxml:5
    override fun onExit(state: Test333State) {
        when (state) {
            is Test333State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test333State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test333State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test333.scxml:5
    override fun executeTransitionActions(
        source: Test333State,
        event: Test333Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
