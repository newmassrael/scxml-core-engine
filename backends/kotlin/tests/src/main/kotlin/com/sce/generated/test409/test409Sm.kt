// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/409/test409.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test409.scxml:7 :: _machine

package com.sce.generated.test409

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test409State : State {
    data object Fail : Test409State
    data object Pass : Test409State
    data object S0 : Test409State
    data object S01 : Test409State
    data object S011 : Test409State
    data object S02 : Test409State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test409Event : Event {
    sealed interface Error : Test409Event {
        data object Execution : Error
    }
    data object Event1 : Test409Event
    data object Timeout : Test409Event
}
// --- State Machine (W3C SCXML) ---

class Test409StateMachine(
) : StateMachineEngine<Test409State, Test409Event>() {

    override val initialState: Test409State = Test409State.S011

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
    override fun parentOf(state: Test409State): Test409State? = when (state) {
        is Test409State.S01 -> Test409State.S0
        is Test409State.S011 -> Test409State.S01
        is Test409State.S02 -> Test409State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test409State): Boolean = when (state) {
        is Test409State.S0, is Test409State.S01 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test409State): Boolean = when (state) {
        is Test409State.Fail, is Test409State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test409State): List<Test409State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test409State): List<EntryTarget<Test409State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test409State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test409State, List<Test409State>> = mapOf(
            Test409State.S0 to listOf(Test409State.S01, Test409State.S02),
            Test409State.S01 to listOf(Test409State.S011),
        )

        val initialTargets: Map<Test409State, List<EntryTarget<Test409State, HistoryId>>> = mapOf(
            Test409State.S0 to listOf(StateTarget(Test409State.S01)),
            Test409State.S01 to listOf(StateTarget(Test409State.S011)),
        )

        val documentInitialTargetList: List<EntryTarget<Test409State, HistoryId>> =
            listOf(StateTarget(Test409State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test409State, HistoryId>(
            Test409State.S0,
            listOf(StateTarget(Test409State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test409State, HistoryId>(
            Test409State.S0,
            listOf(StateTarget(Test409State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s011's transition 0, as the microstep reads it.
        val transitionS011At0 = EnabledTransition<Test409State, HistoryId>(
            Test409State.S011,
            listOf(StateTarget(Test409State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test409State? = when (stateId) {
        "fail" -> Test409State.Fail
        "pass" -> Test409State.Pass
        "s0" -> Test409State.S0
        "s01" -> Test409State.S01
        "s011" -> Test409State.S011
        "s02" -> Test409State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test409State): String = when (state) {
        is Test409State.Fail -> "fail"
        is Test409State.Pass -> "pass"
        is Test409State.S0 -> "s0"
        is Test409State.S01 -> "s01"
        is Test409State.S011 -> "s011"
        is Test409State.S02 -> "s02"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test409State): Int = when (state) {
        is Test409State.Fail -> 5
        is Test409State.Pass -> 4
        is Test409State.S0 -> 0
        is Test409State.S01 -> 1
        is Test409State.S011 -> 2
        is Test409State.S02 -> 3
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test409State,
        event: Test409Event?
    ): EnabledTransition<Test409State, HistoryId>? = when (state) {
        is Test409State.S0 -> when {
            event is Test409Event.Timeout -> transitionS0At0
            event is Test409Event.Event1 -> transitionS0At1
            else -> null
        }
        is Test409State.S011 -> when {
            event == null -> transitionS011At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun onEntry(state: Test409State, isDefaultEntry: Boolean) {
        when (state) {
            is Test409State.Fail -> {
                // SCE-MAP: test409.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test409State.Pass -> {
                // SCE-MAP: test409.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test409State.S0 -> {
                // SCE-MAP: test409.scxml:10 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test409Event.Timeout)
            }
            is Test409State.S01 -> {
                // SCE-MAP: test409.scxml:18 :: s01 :: _state_body
            }
            is Test409State.S011 -> {
                // SCE-MAP: test409.scxml:25 :: s011 :: _state_body
            }
            is Test409State.S02 -> {
                // SCE-MAP: test409.scxml:30 :: s02 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun onExit(state: Test409State) {
        when (state) {
            is Test409State.Fail -> {
                // SCE-MAP: test409.scxml:35 :: fail :: _state_body
            }
            is Test409State.Pass -> {
                // SCE-MAP: test409.scxml:34 :: pass :: _state_body
            }
            is Test409State.S0 -> {
                // SCE-MAP: test409.scxml:10 :: s0 :: _state_body
            }
            is Test409State.S01 -> {
                // SCE-MAP: test409.scxml:18 :: s01 :: _state_body


            if (isStateActive("s011")) {

            raiseInternal(Test409Event.Event1)
            }
            }
            is Test409State.S011 -> {
                // SCE-MAP: test409.scxml:25 :: s011 :: _state_body
            }
            is Test409State.S02 -> {
                // SCE-MAP: test409.scxml:30 :: s02 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test409State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
