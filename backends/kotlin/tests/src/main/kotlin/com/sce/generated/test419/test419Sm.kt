// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/419/test419.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test419.scxml:6 :: _machine

package com.sce.generated.test419

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test419State : State {
    data object Fail : Test419State
    data object Pass : Test419State
    data object S1 : Test419State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test419Event : Event {
    sealed interface Error : Test419Event {
        data object Execution : Error
    }
    data object ExternalEvent : Test419Event
    data object InternalEvent : Test419Event
}
// --- State Machine (W3C SCXML) ---

class Test419StateMachine(
) : StateMachineEngine<Test419State, Test419Event>() {

    override val initialState: Test419State = Test419State.S1

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test419State? = when (stateId) {
        "fail" -> Test419State.Fail
        "pass" -> Test419State.Pass
        "s1" -> Test419State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test419State): String = when (state) {
        is Test419State.Fail -> "fail"
        is Test419State.Pass -> "pass"
        is Test419State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test419State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test419State): Int = when (state) {
        is Test419State.Fail -> 2
        is Test419State.Pass -> 1
        is Test419State.S1 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test419State,
        event: Test419Event
    ): TransitionResult<Test419State> = when (state) {
        is Test419State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test419State
    ): TransitionResult<Test419State> = when (state) {
        is Test419State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test419State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test419State.Pass, Test419State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test419Event
    ): TransitionResult<Test419State> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test419State.Fail, Test419State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test419.scxml:6 :: _machine
    override fun onEntry(state: Test419State, pathChild: Test419State?) {
        when (state) {
            is Test419State.Fail -> {
                // SCE-MAP: test419.scxml:21 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.Pass -> {
                // SCE-MAP: test419.scxml:20 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.S1 -> {
                // SCE-MAP: test419.scxml:8 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test419Event.InternalEvent)


            send(Test419Event.ExternalEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test419.scxml:6 :: _machine
    override fun onExit(state: Test419State) {
        when (state) {
            is Test419State.Fail -> {
                // SCE-MAP: test419.scxml:21 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test419State.Pass -> {
                // SCE-MAP: test419.scxml:20 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test419State.S1 -> {
                // SCE-MAP: test419.scxml:8 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test419.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test419State,
        event: Test419Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
