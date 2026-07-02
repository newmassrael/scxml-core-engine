// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: b5e91c83753cb468c86997c5541ac646288562f682111eb4bbd825060d84bc2e
// generated-at: 1782963882

// GENERATED CODE — DO NOT EDIT
// Source: resources/144/test144.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test144.scxml:6

package com.sce.generated.test144

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test144State : State {
    data object Fail : Test144State
    data object Pass : Test144State
    data object S0 : Test144State
    data object S1 : Test144State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test144Event : Event {
    data object Bar : Test144Event
    data object Foo : Test144Event
}
// --- State Machine (W3C SCXML) ---

class Test144StateMachine(
) : StateMachineEngine<Test144State, Test144Event>() {

    override val initialState: Test144State = Test144State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test144State? = when (stateId) {
        "fail" -> Test144State.Fail
        "pass" -> Test144State.Pass
        "s0" -> Test144State.S0
        "s1" -> Test144State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test144State): String = when (state) {
        is Test144State.Fail -> "fail"
        is Test144State.Pass -> "pass"
        is Test144State.S0 -> "s0"
        is Test144State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test144State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test144State): Int = when (state) {
        is Test144State.Fail -> 3
        is Test144State.Pass -> 2
        is Test144State.S0 -> 0
        is Test144State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test144State,
        event: Test144Event
    ): TransitionResult<Test144State> = when (state) {
        is Test144State.S0 -> processS0(event)
        is Test144State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test144Event
    ): TransitionResult<Test144State> = when {
        event is Test144Event.Foo -> TransitionResult.External(Test144State.S1, Test144State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test144State.Fail, Test144State.S0)
    }

    private fun processS1(
        event: Test144Event
    ): TransitionResult<Test144State> = when {
        event is Test144Event.Bar -> TransitionResult.External(Test144State.Pass, Test144State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test144State.Fail, Test144State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test144.scxml:6
    override fun onEntry(state: Test144State) {
        when (state) {
            is Test144State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test144State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test144State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test144Event.Foo)

            raiseInternal(Test144Event.Bar)
            }
            is Test144State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test144.scxml:6
    override fun onExit(state: Test144State) {
        when (state) {
            is Test144State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test144State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test144State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test144State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test144.scxml:6
    override fun executeTransitionActions(
        source: Test144State,
        event: Test144Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
