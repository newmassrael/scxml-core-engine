// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018

// GENERATED CODE — DO NOT EDIT
// Source: resources/375/test375.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test375.scxml:5

package com.sce.generated.test375

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test375State : State {
    data object Fail : Test375State
    data object Pass : Test375State
    data object S0 : Test375State
    data object S1 : Test375State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test375Event : Event {
    data object Event1 : Test375Event
    data object Event2 : Test375Event
}
// --- State Machine (W3C SCXML) ---

class Test375StateMachine(
) : StateMachineEngine<Test375State, Test375Event>() {

    override val initialState: Test375State = Test375State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test375State? = when (stateId) {
        "fail" -> Test375State.Fail
        "pass" -> Test375State.Pass
        "s0" -> Test375State.S0
        "s1" -> Test375State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test375State): String = when (state) {
        is Test375State.Fail -> "fail"
        is Test375State.Pass -> "pass"
        is Test375State.S0 -> "s0"
        is Test375State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test375State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test375State): Int = when (state) {
        is Test375State.Fail -> 3
        is Test375State.Pass -> 2
        is Test375State.S0 -> 0
        is Test375State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test375State,
        event: Test375Event
    ): TransitionResult<Test375State> = when (state) {
        is Test375State.S0 -> processS0(event)
        is Test375State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test375Event
    ): TransitionResult<Test375State> = when {
        event is Test375Event.Event1 -> TransitionResult.External(Test375State.S1, Test375State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test375State.Fail, Test375State.S0)
    }

    private fun processS1(
        event: Test375Event
    ): TransitionResult<Test375State> = when {
        event is Test375Event.Event2 -> TransitionResult.External(Test375State.Pass, Test375State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test375State.Fail, Test375State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test375.scxml:5
    override fun onEntry(state: Test375State) {
        when (state) {
            is Test375State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                // W3C SCXML 3.8: Onentry block 1/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test375Event.Event1)
                }
                // W3C SCXML 3.8: Onentry block 2/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test375Event.Event2)
                }
            }
            is Test375State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test375.scxml:5
    override fun onExit(state: Test375State) {
        when (state) {
            is Test375State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test375State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test375State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test375State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test375.scxml:5
    override fun executeTransitionActions(
        source: Test375State,
        event: Test375Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
