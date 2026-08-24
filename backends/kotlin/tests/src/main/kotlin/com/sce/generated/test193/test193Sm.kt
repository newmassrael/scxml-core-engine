// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4cbf0ce468f2db0011b4fa010e6c117357964548e492f95e76a21755c70778e3
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/193/test193.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test193.scxml:5 :: _machine

package com.sce.generated.test193

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test193State : State {
    data object Fail : Test193State
    data object Pass : Test193State
    data object S0 : Test193State
    data object S1 : Test193State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test193Event : Event {
    sealed interface Error : Test193Event {
        data object Execution : Error
    }
    data object Event1 : Test193Event
    data object Internal : Test193Event
    data object Timeout : Test193Event
}
// --- State Machine (W3C SCXML) ---

class Test193StateMachine(
) : StateMachineEngine<Test193State, Test193Event>() {

    override val initialState: Test193State = Test193State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test193State? = when (stateId) {
        "fail" -> Test193State.Fail
        "pass" -> Test193State.Pass
        "s0" -> Test193State.S0
        "s1" -> Test193State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test193State): String = when (state) {
        is Test193State.Fail -> "fail"
        is Test193State.Pass -> "pass"
        is Test193State.S0 -> "s0"
        is Test193State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test193State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test193State): Int = when (state) {
        is Test193State.Fail -> 3
        is Test193State.Pass -> 2
        is Test193State.S0 -> 0
        is Test193State.S1 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test193State,
        event: Test193Event
    ): TransitionResult<Test193State> = when (state) {
        is Test193State.S0 -> processS0(event)
        is Test193State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test193Event
    ): TransitionResult<Test193State> = when {
        event is Test193Event.Event1 -> TransitionResult.External(Test193State.Fail, Test193State.S0)

        event is Test193Event.Internal -> TransitionResult.External(Test193State.S1, Test193State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test193Event
    ): TransitionResult<Test193State> = when {
        event is Test193Event.Event1 -> TransitionResult.External(Test193State.Pass, Test193State.S1)

        event is Test193Event.Timeout -> TransitionResult.External(Test193State.Fail, Test193State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test193.scxml:5 :: _machine
    override fun onEntry(state: Test193State, pathChild: Test193State?) {
        when (state) {
            is Test193State.Fail -> {
                // SCE-MAP: test193.scxml:27 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test193State.Pass -> {
                // SCE-MAP: test193.scxml:26 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test193State.S0 -> {
                // SCE-MAP: test193.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test193Event.Internal, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            send(Test193Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))


            scheduleSend("__send_2", 1000L, Test193Event.Timeout)
            }
            is Test193State.S1 -> {
                // SCE-MAP: test193.scxml:20 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test193.scxml:5 :: _machine
    override fun onExit(state: Test193State) {
        when (state) {
            is Test193State.Fail -> {
                // SCE-MAP: test193.scxml:27 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test193State.Pass -> {
                // SCE-MAP: test193.scxml:26 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test193State.S0 -> {
                // SCE-MAP: test193.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test193State.S1 -> {
                // SCE-MAP: test193.scxml:20 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test193.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test193State,
        event: Test193Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
