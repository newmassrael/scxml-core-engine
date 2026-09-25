// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/232/test232.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test232.scxml:5 :: _machine

package com.sce.generated.test232

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test232State : State {
    data object Fail : Test232State
    data object Pass : Test232State
    data object S0 : Test232State
    data object S01 : Test232State
    data object S02 : Test232State
    data object S03 : Test232State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232Event : Event {
    sealed interface Cancel : Test232Event {
        data object Invoke : Cancel
    }
    data object ChildToParent1 : Test232Event
    data object ChildToParent2 : Test232Event
    sealed interface Done : Test232Event {
        data object Invoke : Done
    }
    sealed interface Error : Test232Event {
        data object Execution : Error
    }
    data object Timeout : Test232Event
}
// --- State Machine (W3C SCXML) ---

class Test232StateMachine(
) : StateMachineEngine<Test232State, Test232Event>() {

    override val initialState: Test232State = Test232State.S01

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
    override fun parentOf(state: Test232State): Test232State? = when (state) {
        is Test232State.S01 -> Test232State.S0
        is Test232State.S02 -> Test232State.S0
        is Test232State.S03 -> Test232State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test232State): Boolean = when (state) {
        is Test232State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test232State): Boolean = when (state) {
        is Test232State.Fail, is Test232State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test232State): List<Test232State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test232State): List<EntryTarget<Test232State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test232State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test232State, List<Test232State>> = mapOf(
            Test232State.S0 to listOf(Test232State.S01, Test232State.S02, Test232State.S03),
        )

        val initialTargets: Map<Test232State, List<EntryTarget<Test232State, HistoryId>>> = mapOf(
            Test232State.S0 to listOf(StateTarget(Test232State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test232State, HistoryId>> =
            listOf(StateTarget(Test232State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S0,
            listOf(StateTarget(Test232State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S01,
            listOf(StateTarget(Test232State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S02,
            listOf(StateTarget(Test232State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S03,
            listOf(StateTarget(Test232State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test232State? = when (stateId) {
        "fail" -> Test232State.Fail
        "pass" -> Test232State.Pass
        "s0" -> Test232State.S0
        "s01" -> Test232State.S01
        "s02" -> Test232State.S02
        "s03" -> Test232State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test232State): String = when (state) {
        is Test232State.Fail -> "fail"
        is Test232State.Pass -> "pass"
        is Test232State.S0 -> "s0"
        is Test232State.S01 -> "s01"
        is Test232State.S02 -> "s02"
        is Test232State.S03 -> "s03"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test232State): Int = when (state) {
        is Test232State.Fail -> 5
        is Test232State.Pass -> 4
        is Test232State.S0 -> 0
        is Test232State.S01 -> 1
        is Test232State.S02 -> 2
        is Test232State.S03 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232Event? = when (name) {
        "cancel.invoke" -> Test232Event.Cancel.Invoke
        "childToParent1" -> Test232Event.ChildToParent1
        "childToParent2" -> Test232Event.ChildToParent2
        "done.invoke" -> Test232Event.Done.Invoke
        "error.execution" -> Test232Event.Error.Execution
        "timeout" -> Test232Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232Event): String? = when (event) {
        is Test232Event.Cancel.Invoke -> "cancel.invoke"
        is Test232Event.ChildToParent1 -> "childToParent1"
        is Test232Event.ChildToParent2 -> "childToParent2"
        is Test232Event.Done.Invoke -> "done.invoke"
        is Test232Event.Error.Execution -> "error.execution"
        is Test232Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test232State,
        event: Test232Event?
    ): EnabledTransition<Test232State, HistoryId>? = when (state) {
        is Test232State.S0 -> when {
            event is Test232Event.Timeout -> transitionS0At0
            else -> null
        }
        is Test232State.S01 -> when {
            event is Test232Event.ChildToParent1 -> transitionS01At0
            else -> null
        }
        is Test232State.S02 -> when {
            event is Test232Event.ChildToParent2 -> transitionS02At0
            else -> null
        }
        is Test232State.S03 -> when {
            event is Test232Event.Done.Invoke -> transitionS03At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun onEntry(state: Test232State, isDefaultEntry: Boolean) {
        when (state) {
            is Test232State.Fail -> {
                // SCE-MAP: test232.scxml:42 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.Pass -> {
                // SCE-MAP: test232.scxml:41 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.S0 -> {
                // SCE-MAP: test232.scxml:8 :: s0 :: _state_body


            scheduleSend("__send_0", 3000L, Test232Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test232SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test232Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test232State.S01 -> {
                // SCE-MAP: test232.scxml:27 :: s01 :: _state_body
            }
            is Test232State.S02 -> {
                // SCE-MAP: test232.scxml:31 :: s02 :: _state_body
            }
            is Test232State.S03 -> {
                // SCE-MAP: test232.scxml:35 :: s03 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun onExit(state: Test232State) {
        when (state) {
            is Test232State.Fail -> {
                // SCE-MAP: test232.scxml:42 :: fail :: _state_body
            }
            is Test232State.Pass -> {
                // SCE-MAP: test232.scxml:41 :: pass :: _state_body
            }
            is Test232State.S0 -> {
                // SCE-MAP: test232.scxml:8 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test232State.S01 -> {
                // SCE-MAP: test232.scxml:27 :: s01 :: _state_body
            }
            is Test232State.S02 -> {
                // SCE-MAP: test232.scxml:31 :: s02 :: _state_body
            }
            is Test232State.S03 -> {
                // SCE-MAP: test232.scxml:35 :: s03 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test232State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
