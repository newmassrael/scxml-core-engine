// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/411/test411.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test411.scxml:8 :: _machine

package com.sce.generated.test411

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test411State : State {
    data object Fail : Test411State
    data object Pass : Test411State
    data object S0 : Test411State
    data object S01 : Test411State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test411Event : Event {
    sealed interface Error : Test411Event {
        data object Execution : Error
    }
    data object Event1 : Test411Event
    data object Event2 : Test411Event
    data object Timeout : Test411Event
}
// --- State Machine (W3C SCXML) ---

class Test411StateMachine(
) : StateMachineEngine<Test411State, Test411Event>() {

    override val initialState: Test411State = Test411State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test411State): Test411State? = when (state) {
        is Test411State.S01 -> Test411State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test411State): Boolean = when (state) {
        is Test411State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test411State): Boolean = when (state) {
        is Test411State.Fail, is Test411State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test411State): List<Test411State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test411State): List<EntryTarget<Test411State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test411State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test411State, List<Test411State>> = mapOf(
            Test411State.S0 to listOf(Test411State.S01),
        )

        val initialTargets: Map<Test411State, List<EntryTarget<Test411State, HistoryId>>> = mapOf(
            Test411State.S0 to listOf(StateTarget(Test411State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test411State, HistoryId>> =
            listOf(StateTarget(Test411State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test411State, HistoryId>(
            Test411State.S0,
            listOf(StateTarget(Test411State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test411State, HistoryId>(
            Test411State.S0,
            listOf(StateTarget(Test411State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test411State, HistoryId>(
            Test411State.S0,
            listOf(StateTarget(Test411State.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test411State? = when (stateId) {
        "fail" -> Test411State.Fail
        "pass" -> Test411State.Pass
        "s0" -> Test411State.S0
        "s01" -> Test411State.S01
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test411State): String = when (state) {
        is Test411State.Fail -> "fail"
        is Test411State.Pass -> "pass"
        is Test411State.S0 -> "s0"
        is Test411State.S01 -> "s01"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test411State): Int = when (state) {
        is Test411State.Fail -> 3
        is Test411State.Pass -> 2
        is Test411State.S0 -> 0
        is Test411State.S01 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test411State,
        event: Test411Event?
    ): EnabledTransition<Test411State, HistoryId>? = when (state) {
        is Test411State.S0 -> when {
            event is Test411Event.Timeout -> transitionS0At0
            event is Test411Event.Event1 -> transitionS0At1
            event is Test411Event.Event2 -> transitionS0At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test411.scxml:8 :: _machine
    override fun onEntry(state: Test411State, isDefaultEntry: Boolean) {
        when (state) {
            is Test411State.Fail -> {
                // SCE-MAP: test411.scxml:34 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test411State.Pass -> {
                // SCE-MAP: test411.scxml:33 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test411State.S0 -> {
                // SCE-MAP: test411.scxml:11 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test411Event.Timeout)


            if (isStateActive("s01")) {

            raiseInternal(Test411Event.Event1)
            }
            }
            is Test411State.S01 -> {
                // SCE-MAP: test411.scxml:23 :: s01 :: _state_body


            if (isStateActive("s01")) {

            raiseInternal(Test411Event.Event2)
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test411.scxml:8 :: _machine
    override fun onExit(state: Test411State) {
        when (state) {
            is Test411State.Fail -> {
                // SCE-MAP: test411.scxml:34 :: fail :: _state_body
            }
            is Test411State.Pass -> {
                // SCE-MAP: test411.scxml:33 :: pass :: _state_body
            }
            is Test411State.S0 -> {
                // SCE-MAP: test411.scxml:11 :: s0 :: _state_body
            }
            is Test411State.S01 -> {
                // SCE-MAP: test411.scxml:23 :: s01 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test411.scxml:8 :: _machine
    override fun executeTransitionContent(source: Test411State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
