// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/333/test333.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test333.scxml:5 :: _machine

package com.sce.generated.test333

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test333State : State {
    data object Fail : Test333State
    data object Pass : Test333State
    data object S0 : Test333State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test333Event : Event {
    sealed interface Error : Test333Event {
        data object Execution : Error
    }
    data object Foo : Test333Event
}
// --- State Machine (W3C SCXML) ---

class Test333StateMachine(
) : StateMachineEngine<Test333State, Test333Event>() {

    override val initialState: Test333State = Test333State.S0

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
    override fun isFinalState(state: Test333State): Boolean = when (state) {
        is Test333State.Fail, is Test333State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test333State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test333State, HistoryId>> =
            listOf(StateTarget(Test333State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test333State, HistoryId>(
            Test333State.S0,
            listOf(StateTarget(Test333State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test333State, HistoryId>(
            Test333State.S0,
            listOf(StateTarget(Test333State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test333State? = when (stateId) {
        "fail" -> Test333State.Fail
        "pass" -> Test333State.Pass
        "s0" -> Test333State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test333State): String = when (state) {
        is Test333State.Fail -> "fail"
        is Test333State.Pass -> "pass"
        is Test333State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test333State): Int = when (state) {
        is Test333State.Fail -> 2
        is Test333State.Pass -> 1
        is Test333State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test333State,
        event: Test333Event?
    ): EnabledTransition<Test333State, HistoryId>? = when (state) {
        is Test333State.S0 -> when {
            event is Test333Event.Foo -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test333.scxml:5 :: _machine
    override fun onEntry(state: Test333State, isDefaultEntry: Boolean) {
        when (state) {
            is Test333State.Fail -> {
                // SCE-MAP: test333.scxml:18 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.Pass -> {
                // SCE-MAP: test333.scxml:17 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.S0 -> {
                // SCE-MAP: test333.scxml:7 :: s0 :: _state_body


            send(Test333Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test333.scxml:5 :: _machine
    override fun onExit(state: Test333State) {
        when (state) {
            is Test333State.Fail -> {
                // SCE-MAP: test333.scxml:18 :: fail :: _state_body
            }
            is Test333State.Pass -> {
                // SCE-MAP: test333.scxml:17 :: pass :: _state_body
            }
            is Test333State.S0 -> {
                // SCE-MAP: test333.scxml:7 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test333.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test333State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
