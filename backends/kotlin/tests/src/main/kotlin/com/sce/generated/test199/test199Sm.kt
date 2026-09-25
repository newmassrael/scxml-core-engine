// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/199/test199.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test199.scxml:5 :: _machine

package com.sce.generated.test199

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test199State : State {
    data object Fail : Test199State
    data object Pass : Test199State
    data object S0 : Test199State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test199Event : Event {
    sealed interface Error : Test199Event {
        data object Execution : Error
    }
    data object Event1 : Test199Event
    data object Timeout : Test199Event
}
// --- State Machine (W3C SCXML) ---

class Test199StateMachine(
) : StateMachineEngine<Test199State, Test199Event>() {

    override val initialState: Test199State = Test199State.S0

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
    override fun isFinalState(state: Test199State): Boolean = when (state) {
        is Test199State.Fail, is Test199State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test199State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test199State, HistoryId>> =
            listOf(StateTarget(Test199State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test199State, HistoryId>(
            Test199State.S0,
            listOf(StateTarget(Test199State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test199State, HistoryId>(
            Test199State.S0,
            listOf(StateTarget(Test199State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test199State? = when (stateId) {
        "fail" -> Test199State.Fail
        "pass" -> Test199State.Pass
        "s0" -> Test199State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test199State): String = when (state) {
        is Test199State.Fail -> "fail"
        is Test199State.Pass -> "pass"
        is Test199State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test199State): Int = when (state) {
        is Test199State.Fail -> 2
        is Test199State.Pass -> 1
        is Test199State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test199State,
        event: Test199Event?
    ): EnabledTransition<Test199State, HistoryId>? = when (state) {
        is Test199State.S0 -> when {
            event is Test199Event.Error.Execution -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test199.scxml:5 :: _machine
    override fun onEntry(state: Test199State, isDefaultEntry: Boolean) {
        when (state) {
            is Test199State.Fail -> {
                // SCE-MAP: test199.scxml:20 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.Pass -> {
                // SCE-MAP: test199.scxml:19 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.S0 -> {
                // SCE-MAP: test199.scxml:7 :: s0 :: _state_body


            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raisePlatformError(Test199Event.Error.Execution, "<send type='unsupported_type'> names a processor this platform does not support", "__send_0")
            return  // W3C SCXML 5.10: Stop subsequent executable content


            send(Test199Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test199.scxml:5 :: _machine
    override fun onExit(state: Test199State) {
        when (state) {
            is Test199State.Fail -> {
                // SCE-MAP: test199.scxml:20 :: fail :: _state_body
            }
            is Test199State.Pass -> {
                // SCE-MAP: test199.scxml:19 :: pass :: _state_body
            }
            is Test199State.S0 -> {
                // SCE-MAP: test199.scxml:7 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test199.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test199State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
