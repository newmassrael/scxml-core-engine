// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d6df7c5cb569a8142d0ee296b73fd46e2cbd91d66a31cab131337d70b3fd380b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/577/test577.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test577.scxml:5 :: _machine

package com.sce.generated.test577

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test577State : State {
    data object Fail : Test577State
    data object Pass : Test577State
    data object S0 : Test577State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test577Event : Event {
    sealed interface Error : Test577Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Event1 : Test577Event
    data object Test : Test577Event
}
// --- State Machine (W3C SCXML) ---

class Test577StateMachine(
) : StateMachineEngine<Test577State, Test577Event>() {

    override val initialState: Test577State = Test577State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test577State? = when (stateId) {
        "fail" -> Test577State.Fail
        "pass" -> Test577State.Pass
        "s0" -> Test577State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test577State): String = when (state) {
        is Test577State.Fail -> "fail"
        is Test577State.Pass -> "pass"
        is Test577State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test577State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test577State): Int = when (state) {
        is Test577State.Fail -> 2
        is Test577State.Pass -> 1
        is Test577State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test577State,
        event: Test577Event
    ): TransitionResult<Test577State> = when (state) {
        is Test577State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test577Event
    ): TransitionResult<Test577State> = when {
        event is Test577Event.Error.Communication -> TransitionResult.External(Test577State.Pass, Test577State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test577State.Fail, Test577State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test577.scxml:5 :: _machine
    override fun onEntry(state: Test577State, pathChild: Test577State?) {
        when (state) {
            is Test577State.Fail -> {
                // SCE-MAP: test577.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.Pass -> {
                // SCE-MAP: test577.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.S0 -> {
                // SCE-MAP: test577.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test577Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            // W3C SCXML C.2 (test577): BasicHTTP requires target, missing raises error.communication
            raisePlatformError(Test577Event.Error.Communication, "<send> over BasicHTTPEventProcessor has no target to post to")
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test577.scxml:5 :: _machine
    override fun onExit(state: Test577State) {
        when (state) {
            is Test577State.Fail -> {
                // SCE-MAP: test577.scxml:23 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test577State.Pass -> {
                // SCE-MAP: test577.scxml:22 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test577State.S0 -> {
                // SCE-MAP: test577.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test577.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test577State,
        event: Test577Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
