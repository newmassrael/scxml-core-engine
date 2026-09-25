// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test419State): Boolean = when (state) {
        is Test419State.Fail, is Test419State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test419State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test419State, HistoryId>> =
            listOf(StateTarget(Test419State.S1))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test419State, HistoryId>(
            Test419State.S1,
            listOf(StateTarget(Test419State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test419State, HistoryId>(
            Test419State.S1,
            listOf(StateTarget(Test419State.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test419State): Int = when (state) {
        is Test419State.Fail -> 2
        is Test419State.Pass -> 1
        is Test419State.S1 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test419State,
        event: Test419Event?
    ): EnabledTransition<Test419State, HistoryId>? = when (state) {
        is Test419State.S1 -> when {
            event != null -> transitionS1At0
            else -> transitionS1At1
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test419.scxml:6 :: _machine
    override fun onEntry(state: Test419State, isDefaultEntry: Boolean) {
        when (state) {
            is Test419State.Fail -> {
                // SCE-MAP: test419.scxml:21 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.Pass -> {
                // SCE-MAP: test419.scxml:20 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.S1 -> {
                // SCE-MAP: test419.scxml:8 :: s1 :: _state_body

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
            }
            is Test419State.Pass -> {
                // SCE-MAP: test419.scxml:20 :: pass :: _state_body
            }
            is Test419State.S1 -> {
                // SCE-MAP: test419.scxml:8 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test419.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test419State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
