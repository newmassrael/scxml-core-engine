// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/377/test377.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test377.scxml:5 :: _machine

package com.sce.generated.test377

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test377State : State {
    data object Fail : Test377State
    data object Pass : Test377State
    data object S0 : Test377State
    data object S1 : Test377State
    data object S2 : Test377State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test377Event : Event {
    data object Event1 : Test377Event
    data object Event2 : Test377Event
}
// --- State Machine (W3C SCXML) ---

class Test377StateMachine(
) : StateMachineEngine<Test377State, Test377Event>() {

    override val initialState: Test377State = Test377State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test377State? = when (stateId) {
        "fail" -> Test377State.Fail
        "pass" -> Test377State.Pass
        "s0" -> Test377State.S0
        "s1" -> Test377State.S1
        "s2" -> Test377State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test377State): String = when (state) {
        is Test377State.Fail -> "fail"
        is Test377State.Pass -> "pass"
        is Test377State.S0 -> "s0"
        is Test377State.S1 -> "s1"
        is Test377State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test377State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test377State): Int = when (state) {
        is Test377State.Fail -> 4
        is Test377State.Pass -> 3
        is Test377State.S0 -> 0
        is Test377State.S1 -> 1
        is Test377State.S2 -> 2
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test377State,
        event: Test377Event
    ): TransitionResult<Test377State> = when (state) {
        is Test377State.S1 -> processS1(event)
        is Test377State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test377State
    ): TransitionResult<Test377State> = when (state) {
        is Test377State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test377State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test377State.S1, Test377State.S0)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test377Event
    ): TransitionResult<Test377State> = when {
        event is Test377Event.Event1 -> TransitionResult.External(Test377State.S2, Test377State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test377State.Fail, Test377State.S1)
    }

    private fun processS2(
        event: Test377Event
    ): TransitionResult<Test377State> = when {
        event is Test377Event.Event2 -> TransitionResult.External(Test377State.Pass, Test377State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test377State.Fail, Test377State.S2)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun onEntry(state: Test377State, pathChild: Test377State?) {
        when (state) {
            is Test377State.Fail -> {
                // SCE-MAP: test377.scxml:34 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test377State.Pass -> {
                // SCE-MAP: test377.scxml:33 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test377State.S0 -> {
                // SCE-MAP: test377.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test377State.S1 -> {
                // SCE-MAP: test377.scxml:20 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test377State.S2 -> {
                // SCE-MAP: test377.scxml:27 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun onExit(state: Test377State) {
        when (state) {
            is Test377State.Fail -> {
                // SCE-MAP: test377.scxml:34 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test377State.Pass -> {
                // SCE-MAP: test377.scxml:33 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test377State.S0 -> {
                // SCE-MAP: test377.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
                // W3C SCXML 3.9: Onexit block 1/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test377Event.Event1)
                }
                // W3C SCXML 3.9: Onexit block 2/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test377Event.Event2)
                }
            }
            is Test377State.S1 -> {
                // SCE-MAP: test377.scxml:20 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test377State.S2 -> {
                // SCE-MAP: test377.scxml:27 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test377State,
        event: Test377Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
