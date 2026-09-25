// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/421/test421.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test421.scxml:7 :: _machine

package com.sce.generated.test421

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test421State : State {
    data object Fail : Test421State
    data object Pass : Test421State
    data object S1 : Test421State
    data object S11 : Test421State
    data object S12 : Test421State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test421Event : Event {
    sealed interface Error : Test421Event {
        data object Execution : Error
    }
    data object ExternalEvent : Test421Event
    data object InternalEvent1 : Test421Event
    data object InternalEvent2 : Test421Event
    data object InternalEvent3 : Test421Event
    data object InternalEvent4 : Test421Event
}
// --- State Machine (W3C SCXML) ---

class Test421StateMachine(
) : StateMachineEngine<Test421State, Test421Event>() {

    override val initialState: Test421State = Test421State.S11

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test421State): Test421State? = when (state) {
        is Test421State.S11 -> Test421State.S1
        is Test421State.S12 -> Test421State.S1
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test421State): Boolean = when (state) {
        is Test421State.S1 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test421State): Boolean = when (state) {
        is Test421State.Fail, is Test421State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test421State): List<Test421State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test421State): List<EntryTarget<Test421State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test421State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test421State, List<Test421State>> = mapOf(
            Test421State.S1 to listOf(Test421State.S11, Test421State.S12),
        )

        val initialTargets: Map<Test421State, List<EntryTarget<Test421State, HistoryId>>> = mapOf(
            Test421State.S1 to listOf(StateTarget(Test421State.S11)),
        )

        val documentInitialTargetList: List<EntryTarget<Test421State, HistoryId>> =
            listOf(StateTarget(Test421State.S1))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test421State, HistoryId>(
            Test421State.S1,
            listOf(StateTarget(Test421State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11's transition 0, as the microstep reads it.
        val transitionS11At0 = EnabledTransition<Test421State, HistoryId>(
            Test421State.S11,
            listOf(StateTarget(Test421State.S12)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s12's transition 0, as the microstep reads it.
        val transitionS12At0 = EnabledTransition<Test421State, HistoryId>(
            Test421State.S12,
            listOf(StateTarget(Test421State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test421State? = when (stateId) {
        "fail" -> Test421State.Fail
        "pass" -> Test421State.Pass
        "s1" -> Test421State.S1
        "s11" -> Test421State.S11
        "s12" -> Test421State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test421State): String = when (state) {
        is Test421State.Fail -> "fail"
        is Test421State.Pass -> "pass"
        is Test421State.S1 -> "s1"
        is Test421State.S11 -> "s11"
        is Test421State.S12 -> "s12"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test421State): Int = when (state) {
        is Test421State.Fail -> 4
        is Test421State.Pass -> 3
        is Test421State.S1 -> 0
        is Test421State.S11 -> 1
        is Test421State.S12 -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test421State,
        event: Test421Event?
    ): EnabledTransition<Test421State, HistoryId>? = when (state) {
        is Test421State.S1 -> when {
            event is Test421Event.ExternalEvent -> transitionS1At0
            else -> null
        }
        is Test421State.S11 -> when {
            event is Test421Event.InternalEvent3 -> transitionS11At0
            else -> null
        }
        is Test421State.S12 -> when {
            event is Test421Event.InternalEvent4 -> transitionS12At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test421.scxml:7 :: _machine
    override fun onEntry(state: Test421State, isDefaultEntry: Boolean) {
        when (state) {
            is Test421State.Fail -> {
                // SCE-MAP: test421.scxml:32 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test421State.Pass -> {
                // SCE-MAP: test421.scxml:31 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test421State.S1 -> {
                // SCE-MAP: test421.scxml:9 :: s1 :: _state_body


            send(Test421Event.ExternalEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))

            raiseInternal(Test421Event.InternalEvent1)

            raiseInternal(Test421Event.InternalEvent2)

            raiseInternal(Test421Event.InternalEvent3)

            raiseInternal(Test421Event.InternalEvent4)
            }
            is Test421State.S11 -> {
                // SCE-MAP: test421.scxml:20 :: s11 :: _state_body
            }
            is Test421State.S12 -> {
                // SCE-MAP: test421.scxml:24 :: s12 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test421.scxml:7 :: _machine
    override fun onExit(state: Test421State) {
        when (state) {
            is Test421State.Fail -> {
                // SCE-MAP: test421.scxml:32 :: fail :: _state_body
            }
            is Test421State.Pass -> {
                // SCE-MAP: test421.scxml:31 :: pass :: _state_body
            }
            is Test421State.S1 -> {
                // SCE-MAP: test421.scxml:9 :: s1 :: _state_body
            }
            is Test421State.S11 -> {
                // SCE-MAP: test421.scxml:20 :: s11 :: _state_body
            }
            is Test421State.S12 -> {
                // SCE-MAP: test421.scxml:24 :: s12 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test421.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test421State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
