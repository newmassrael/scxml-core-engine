// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c11ce025286de32d15ba70522b50fb24cf722356167a9d021470bd1434f2dd9a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/208/test208.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test208.scxml:6 :: _machine

package com.sce.generated.test208

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test208State : State {
    data object Fail : Test208State
    data object Pass : Test208State
    data object S0 : Test208State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test208Event : Event {
    sealed interface Error : Test208Event {
        data object Execution : Error
    }
    data object Event1 : Test208Event
    data object Event2 : Test208Event
}
// --- State Machine (W3C SCXML) ---

class Test208StateMachine(
) : StateMachineEngine<Test208State, Test208Event>() {

    override val initialState: Test208State = Test208State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test208State? = when (stateId) {
        "fail" -> Test208State.Fail
        "pass" -> Test208State.Pass
        "s0" -> Test208State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test208State): String = when (state) {
        is Test208State.Fail -> "fail"
        is Test208State.Pass -> "pass"
        is Test208State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test208State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test208State): Int = when (state) {
        is Test208State.Fail -> 2
        is Test208State.Pass -> 1
        is Test208State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test208State,
        event: Test208Event
    ): TransitionResult<Test208State> = when (state) {
        is Test208State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test208Event
    ): TransitionResult<Test208State> = when {
        event is Test208Event.Event2 -> TransitionResult.External(Test208State.Pass, Test208State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test208State.Fail, Test208State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun onEntry(state: Test208State, pathChild: Test208State?) {
        when (state) {
            is Test208State.Fail -> {
                // SCE-MAP: test208.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.Pass -> {
                // SCE-MAP: test208.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.S0 -> {
                // SCE-MAP: test208.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("foo", 1000L, Test208Event.Event1)


            scheduleSend("__send_0", 1500L, Test208Event.Event2)


            cancelSend("foo")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun onExit(state: Test208State) {
        when (state) {
            is Test208State.Fail -> {
                // SCE-MAP: test208.scxml:23 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test208State.Pass -> {
                // SCE-MAP: test208.scxml:22 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test208State.S0 -> {
                // SCE-MAP: test208.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test208State,
        event: Test208Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
