// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test194State): Boolean = when (state) {
        is Test194State.Fail, is Test194State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test194State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test194State, HistoryId>> =
            listOf(StateTarget(Test194State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test194State, HistoryId>(
            Test194State.S0,
            listOf(StateTarget(Test194State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test194State, HistoryId>(
            Test194State.S0,
            listOf(StateTarget(Test194State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test194State): Int = when (state) {
        is Test194State.Fail -> 2
        is Test194State.Pass -> 1
        is Test194State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test194State,
        event: Test194Event?
    ): EnabledTransition<Test194State, HistoryId>? = when (state) {
        is Test194State.S0 -> when {
            event is Test194Event.Error.Execution -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test194.scxml:6 :: _machine
    override fun onEntry(state: Test194State, isDefaultEntry: Boolean) {
        when (state) {
            is Test194State.Fail -> {
                // SCE-MAP: test194.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.Pass -> {
                // SCE-MAP: test194.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.S0 -> {
                // SCE-MAP: test194.scxml:9 :: s0 :: _state_body


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
            }
            is Test194State.Pass -> {
                // SCE-MAP: test194.scxml:23 :: pass :: _state_body
            }
            is Test194State.S0 -> {
                // SCE-MAP: test194.scxml:9 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test194.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test194State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
