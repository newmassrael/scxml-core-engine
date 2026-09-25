// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/252/test252.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test252.scxml:7 :: _machine

package com.sce.generated.test252

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test252State : State {
    data object Fail : Test252State
    data object Pass : Test252State
    data object S0 : Test252State
    data object S01 : Test252State
    data object S02 : Test252State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test252Event : Event {
    sealed interface Cancel : Test252Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test252Event
    sealed interface Done : Test252Event {
        data object Invoke : Done
    }
    sealed interface Error : Test252Event {
        data object Execution : Error
    }
    data object Foo : Test252Event
    data object Timeout : Test252Event
}
// --- State Machine (W3C SCXML) ---

class Test252StateMachine(
) : StateMachineEngine<Test252State, Test252Event>() {

    override val initialState: Test252State = Test252State.S01

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
    override fun parentOf(state: Test252State): Test252State? = when (state) {
        is Test252State.S01 -> Test252State.S0
        is Test252State.S02 -> Test252State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test252State): Boolean = when (state) {
        is Test252State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test252State): Boolean = when (state) {
        is Test252State.Fail, is Test252State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test252State): List<Test252State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test252State): List<EntryTarget<Test252State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test252State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test252State, List<Test252State>> = mapOf(
            Test252State.S0 to listOf(Test252State.S01, Test252State.S02),
        )

        val initialTargets: Map<Test252State, List<EntryTarget<Test252State, HistoryId>>> = mapOf(
            Test252State.S0 to listOf(StateTarget(Test252State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test252State, HistoryId>> =
            listOf(StateTarget(Test252State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test252State, HistoryId>(
            Test252State.S0,
            listOf(StateTarget(Test252State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test252State, HistoryId>(
            Test252State.S0,
            listOf(StateTarget(Test252State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test252State, HistoryId>(
            Test252State.S0,
            listOf(StateTarget(Test252State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test252State, HistoryId>(
            Test252State.S01,
            listOf(StateTarget(Test252State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test252State? = when (stateId) {
        "fail" -> Test252State.Fail
        "pass" -> Test252State.Pass
        "s0" -> Test252State.S0
        "s01" -> Test252State.S01
        "s02" -> Test252State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test252State): String = when (state) {
        is Test252State.Fail -> "fail"
        is Test252State.Pass -> "pass"
        is Test252State.S0 -> "s0"
        is Test252State.S01 -> "s01"
        is Test252State.S02 -> "s02"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test252State): Int = when (state) {
        is Test252State.Fail -> 4
        is Test252State.Pass -> 3
        is Test252State.S0 -> 0
        is Test252State.S01 -> 1
        is Test252State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test252Event? = when (name) {
        "cancel.invoke" -> Test252Event.Cancel.Invoke
        "childToParent" -> Test252Event.ChildToParent
        "done.invoke" -> Test252Event.Done.Invoke
        "error.execution" -> Test252Event.Error.Execution
        "foo" -> Test252Event.Foo
        "timeout" -> Test252Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test252Event): String? = when (event) {
        is Test252Event.Cancel.Invoke -> "cancel.invoke"
        is Test252Event.ChildToParent -> "childToParent"
        is Test252Event.Done.Invoke -> "done.invoke"
        is Test252Event.Error.Execution -> "error.execution"
        is Test252Event.Foo -> "foo"
        is Test252Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test252State,
        event: Test252Event?
    ): EnabledTransition<Test252State, HistoryId>? = when (state) {
        is Test252State.S0 -> when {
            event is Test252Event.Timeout -> transitionS0At0
            event is Test252Event.ChildToParent -> transitionS0At1
            event is Test252Event.Done.Invoke -> transitionS0At2
            else -> null
        }
        is Test252State.S01 -> when {
            event is Test252Event.Foo -> transitionS01At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun onEntry(state: Test252State, isDefaultEntry: Boolean) {
        when (state) {
            is Test252State.Fail -> {
                // SCE-MAP: test252.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test252State.Pass -> {
                // SCE-MAP: test252.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test252State.S0 -> {
                // SCE-MAP: test252.scxml:10 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test252Event.Timeout)
            }
            is Test252State.S01 -> {
                // SCE-MAP: test252.scxml:19 :: s01 :: _state_body


            send(Test252Event.Foo, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test252SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test252Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test252State.S02 -> {
                // SCE-MAP: test252.scxml:45 :: s02 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun onExit(state: Test252State) {
        when (state) {
            is Test252State.Fail -> {
                // SCE-MAP: test252.scxml:50 :: fail :: _state_body
            }
            is Test252State.Pass -> {
                // SCE-MAP: test252.scxml:49 :: pass :: _state_body
            }
            is Test252State.S0 -> {
                // SCE-MAP: test252.scxml:10 :: s0 :: _state_body
            }
            is Test252State.S01 -> {
                // SCE-MAP: test252.scxml:19 :: s01 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test252State.S02 -> {
                // SCE-MAP: test252.scxml:45 :: s02 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test252State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
