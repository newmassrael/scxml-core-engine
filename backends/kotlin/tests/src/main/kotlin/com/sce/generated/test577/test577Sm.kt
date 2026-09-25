// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test577State): Boolean = when (state) {
        is Test577State.Fail, is Test577State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test577State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test577State, HistoryId>> =
            listOf(StateTarget(Test577State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test577State, HistoryId>(
            Test577State.S0,
            listOf(StateTarget(Test577State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test577State, HistoryId>(
            Test577State.S0,
            listOf(StateTarget(Test577State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test577State): Int = when (state) {
        is Test577State.Fail -> 2
        is Test577State.Pass -> 1
        is Test577State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test577State,
        event: Test577Event?
    ): EnabledTransition<Test577State, HistoryId>? = when (state) {
        is Test577State.S0 -> when {
            event is Test577Event.Error.Communication -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test577.scxml:5 :: _machine
    override fun onEntry(state: Test577State, isDefaultEntry: Boolean) {
        when (state) {
            is Test577State.Fail -> {
                // SCE-MAP: test577.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.Pass -> {
                // SCE-MAP: test577.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.S0 -> {
                // SCE-MAP: test577.scxml:8 :: s0 :: _state_body


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
            }
            is Test577State.Pass -> {
                // SCE-MAP: test577.scxml:22 :: pass :: _state_body
            }
            is Test577State.S0 -> {
                // SCE-MAP: test577.scxml:8 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test577.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test577State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
