// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/347/test347.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test347.scxml:6 :: _machine

package com.sce.generated.test347

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test347State : State {
    data object Fail : Test347State
    data object Pass : Test347State
    data object S0 : Test347State
    data object S01 : Test347State
    data object S02 : Test347State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test347Event : Event {
    sealed interface Cancel : Test347Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test347Event
    sealed interface Done : Test347Event {
        data object Invoke : Done
    }
    sealed interface Error : Test347Event {
        data object Self : Error
        data object Execution : Error
    }
    data object ParentToChild : Test347Event
    data object Timeout : Test347Event
}
// --- State Machine (W3C SCXML) ---

class Test347StateMachine(
) : StateMachineEngine<Test347State, Test347Event>() {

    override val initialState: Test347State = Test347State.S01

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
    override fun parentOf(state: Test347State): Test347State? = when (state) {
        is Test347State.S01 -> Test347State.S0
        is Test347State.S02 -> Test347State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test347State): Boolean = when (state) {
        is Test347State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test347State): Boolean = when (state) {
        is Test347State.Fail, is Test347State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test347State): List<Test347State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test347State): List<EntryTarget<Test347State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test347State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test347State, List<Test347State>> = mapOf(
            Test347State.S0 to listOf(Test347State.S01, Test347State.S02),
        )

        val initialTargets: Map<Test347State, List<EntryTarget<Test347State, HistoryId>>> = mapOf(
            Test347State.S0 to listOf(StateTarget(Test347State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test347State, HistoryId>> =
            listOf(StateTarget(Test347State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S0,
            listOf(StateTarget(Test347State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S01,
            listOf(StateTarget(Test347State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S02,
            listOf(StateTarget(Test347State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S02,
            listOf(StateTarget(Test347State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test347State? = when (stateId) {
        "fail" -> Test347State.Fail
        "pass" -> Test347State.Pass
        "s0" -> Test347State.S0
        "s01" -> Test347State.S01
        "s02" -> Test347State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test347State): String = when (state) {
        is Test347State.Fail -> "fail"
        is Test347State.Pass -> "pass"
        is Test347State.S0 -> "s0"
        is Test347State.S01 -> "s01"
        is Test347State.S02 -> "s02"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test347State): Int = when (state) {
        is Test347State.Fail -> 4
        is Test347State.Pass -> 3
        is Test347State.S0 -> 0
        is Test347State.S01 -> 1
        is Test347State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test347Event? = when (name) {
        "cancel.invoke" -> Test347Event.Cancel.Invoke
        "childToParent" -> Test347Event.ChildToParent
        "done.invoke" -> Test347Event.Done.Invoke
        "error" -> Test347Event.Error.Self
        "error.execution" -> Test347Event.Error.Execution
        "parentToChild" -> Test347Event.ParentToChild
        "timeout" -> Test347Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test347Event): String? = when (event) {
        is Test347Event.Cancel.Invoke -> "cancel.invoke"
        is Test347Event.ChildToParent -> "childToParent"
        is Test347Event.Done.Invoke -> "done.invoke"
        is Test347Event.Error.Self -> "error"
        is Test347Event.Error.Execution -> "error.execution"
        is Test347Event.ParentToChild -> "parentToChild"
        is Test347Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test347State,
        event: Test347Event?
    ): EnabledTransition<Test347State, HistoryId>? = when (state) {
        is Test347State.S0 -> when {
            event is Test347Event.Timeout -> transitionS0At0
            else -> null
        }
        is Test347State.S01 -> when {
            event is Test347Event.ChildToParent -> transitionS01At0
            else -> null
        }
        is Test347State.S02 -> when {
            event is Test347Event.Done.Invoke -> transitionS02At0
            (event is Test347Event.Error || event is Test347Event.Error.Execution) -> transitionS02At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun onEntry(state: Test347State, isDefaultEntry: Boolean) {
        when (state) {
            is Test347State.Fail -> {
                // SCE-MAP: test347.scxml:42 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test347State.Pass -> {
                // SCE-MAP: test347.scxml:41 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test347State.S0 -> {
                // SCE-MAP: test347.scxml:9 :: s0 :: _state_body


            scheduleSend("__send_0", 20000L, Test347Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.child"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test347SceSynthInvokeChildStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("child", childSM, false, Test347Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test347State.S01 -> {
                // SCE-MAP: test347.scxml:28 :: s01 :: _state_body
            }
            is Test347State.S02 -> {
                // SCE-MAP: test347.scxml:32 :: s02 :: _state_body


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("child", "parentToChild")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun onExit(state: Test347State) {
        when (state) {
            is Test347State.Fail -> {
                // SCE-MAP: test347.scxml:42 :: fail :: _state_body
            }
            is Test347State.Pass -> {
                // SCE-MAP: test347.scxml:41 :: pass :: _state_body
            }
            is Test347State.S0 -> {
                // SCE-MAP: test347.scxml:9 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("child")
            }
            is Test347State.S01 -> {
                // SCE-MAP: test347.scxml:28 :: s01 :: _state_body
            }
            is Test347State.S02 -> {
                // SCE-MAP: test347.scxml:32 :: s02 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test347State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
