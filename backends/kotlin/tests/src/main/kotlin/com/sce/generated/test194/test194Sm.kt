// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 35c8283af859855fefb53b36dbcc38c1c549511d8a5bf7a3250f4215fef24b75
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/194/test194.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test194.scxml:6 :: _machine

package com.sce.generated.test194

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test194State : State {
    data object Fail : Test194State
    data object Pass : Test194State
    data object S0 : Test194State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test194Event : Event {
    sealed interface Error : Test194Event {
        data object Execution : Error
    }
    data object Event2 : Test194Event
    data object Timeout : Test194Event
}
// --- State Machine (W3C SCXML) ---

class Test194StateMachine(
) : StateMachineEngine<Test194State, Test194Event>() {

    override val initialState: Test194State = Test194State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test194State? = when (stateId) {
        "fail" -> Test194State.Fail
        "pass" -> Test194State.Pass
        "s0" -> Test194State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test194State): String = when (state) {
        is Test194State.Fail -> "fail"
        is Test194State.Pass -> "pass"
        is Test194State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test194State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test194State): Int = when (state) {
        is Test194State.Fail -> 2
        is Test194State.Pass -> 1
        is Test194State.S0 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test194State,
        event: Test194Event
    ): TransitionResult<Test194State> = when (state) {
        is Test194State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test194Event
    ): TransitionResult<Test194State> = when {
        event is Test194Event.Error.Execution -> TransitionResult.External(Test194State.Pass, Test194State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test194State.Fail, Test194State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test194.scxml:6 :: _machine
    override fun onEntry(state: Test194State, pathChild: Test194State?) {
        when (state) {
            is Test194State.Fail -> {
                // SCE-MAP: test194.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.Pass -> {
                // SCE-MAP: test194.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.S0 -> {
                // SCE-MAP: test194.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 6.2 (test194): Invalid target raises error.execution
            raisePlatformError(Test194Event.Error.Execution, "<send target='!invalid'> is not a target this processor can address", "__send_0")
            return  // W3C SCXML 5.10: Stop subsequent executable content


            send(Test194Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test194.scxml:6 :: _machine
    override fun onExit(state: Test194State) {
        when (state) {
            is Test194State.Fail -> {
                // SCE-MAP: test194.scxml:24 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test194State.Pass -> {
                // SCE-MAP: test194.scxml:23 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test194State.S0 -> {
                // SCE-MAP: test194.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test194.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test194State,
        event: Test194Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
