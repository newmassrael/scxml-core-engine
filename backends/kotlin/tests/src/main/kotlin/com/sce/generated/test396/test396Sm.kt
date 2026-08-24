// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 401d2ad22cf222caeef0633edc48b3c3fd2090ab46bb9a2c354f5be833096227
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/396/test396.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test396.scxml:5 :: _machine

package com.sce.generated.test396

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test396State : State {
    data object Fail : Test396State
    data object Pass : Test396State
    data object S0 : Test396State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test396Event : Event {
    data object Foo : Test396Event
}
// --- State Machine (W3C SCXML) ---

class Test396StateMachine(
) : StateMachineEngine<Test396State, Test396Event>() {

    override val initialState: Test396State = Test396State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test396State? = when (stateId) {
        "fail" -> Test396State.Fail
        "pass" -> Test396State.Pass
        "s0" -> Test396State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test396State): String = when (state) {
        is Test396State.Fail -> "fail"
        is Test396State.Pass -> "pass"
        is Test396State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test396State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test396State): Int = when (state) {
        is Test396State.Fail -> 2
        is Test396State.Pass -> 1
        is Test396State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test396State,
        event: Test396Event
    ): TransitionResult<Test396State> = when (state) {
        is Test396State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test396Event
    ): TransitionResult<Test396State> = when {
        event is Test396Event.Foo -> TransitionResult.External(Test396State.Pass, Test396State.S0)

        event is Test396Event.Foo -> TransitionResult.External(Test396State.Fail, Test396State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun onEntry(state: Test396State, pathChild: Test396State?) {
        when (state) {
            is Test396State.Fail -> {
                // SCE-MAP: test396.scxml:19 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.Pass -> {
                // SCE-MAP: test396.scxml:18 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.S0 -> {
                // SCE-MAP: test396.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test396Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun onExit(state: Test396State) {
        when (state) {
            is Test396State.Fail -> {
                // SCE-MAP: test396.scxml:19 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test396State.Pass -> {
                // SCE-MAP: test396.scxml:18 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test396State.S0 -> {
                // SCE-MAP: test396.scxml:7 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test396State,
        event: Test396Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
