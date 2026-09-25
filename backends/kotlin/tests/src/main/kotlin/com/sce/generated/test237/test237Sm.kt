// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/237/test237.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test237.scxml:8 :: _machine

package com.sce.generated.test237

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test237State : State {
    data object Fail : Test237State
    data object Pass : Test237State
    data object S0 : Test237State
    data object S1 : Test237State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test237Event : Event {
    sealed interface Cancel : Test237Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test237Event {
        data object Invoke : Done
    }
    sealed interface Error : Test237Event {
        data object Execution : Error
    }
    data object Timeout1 : Test237Event
    data object Timeout2 : Test237Event
}
// --- State Machine (W3C SCXML) ---

class Test237StateMachine(
) : StateMachineEngine<Test237State, Test237Event>() {

    override val initialState: Test237State = Test237State.S0

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test237State): Boolean = when (state) {
        is Test237State.Fail, is Test237State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test237State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test237State, HistoryId>> =
            listOf(StateTarget(Test237State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test237State, HistoryId>(
            Test237State.S0,
            listOf(StateTarget(Test237State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test237State, HistoryId>(
            Test237State.S1,
            listOf(StateTarget(Test237State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test237State, HistoryId>(
            Test237State.S1,
            listOf(StateTarget(Test237State.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test237State? = when (stateId) {
        "fail" -> Test237State.Fail
        "pass" -> Test237State.Pass
        "s0" -> Test237State.S0
        "s1" -> Test237State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test237State): String = when (state) {
        is Test237State.Fail -> "fail"
        is Test237State.Pass -> "pass"
        is Test237State.S0 -> "s0"
        is Test237State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test237State): Int = when (state) {
        is Test237State.Fail -> 3
        is Test237State.Pass -> 2
        is Test237State.S0 -> 0
        is Test237State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test237Event? = when (name) {
        "cancel.invoke" -> Test237Event.Cancel.Invoke
        "done.invoke" -> Test237Event.Done.Invoke
        "error.execution" -> Test237Event.Error.Execution
        "timeout1" -> Test237Event.Timeout1
        "timeout2" -> Test237Event.Timeout2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test237Event): String? = when (event) {
        is Test237Event.Cancel.Invoke -> "cancel.invoke"
        is Test237Event.Done.Invoke -> "done.invoke"
        is Test237Event.Error.Execution -> "error.execution"
        is Test237Event.Timeout1 -> "timeout1"
        is Test237Event.Timeout2 -> "timeout2"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test237State,
        event: Test237Event?
    ): EnabledTransition<Test237State, HistoryId>? = when (state) {
        is Test237State.S0 -> when {
            event is Test237Event.Timeout1 -> transitionS0At0
            else -> null
        }
        is Test237State.S1 -> when {
            event is Test237Event.Done.Invoke -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test237.scxml:8 :: _machine
    override fun onEntry(state: Test237State, isDefaultEntry: Boolean) {
        when (state) {
            is Test237State.Fail -> {
                // SCE-MAP: test237.scxml:44 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test237State.Pass -> {
                // SCE-MAP: test237.scxml:43 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test237State.S0 -> {
                // SCE-MAP: test237.scxml:11 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test237Event.Timeout1)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test237SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test237Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test237State.S1 -> {
                // SCE-MAP: test237.scxml:34 :: s1 :: _state_body


            scheduleSend("__send_1", 1500L, Test237Event.Timeout2)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test237.scxml:8 :: _machine
    override fun onExit(state: Test237State) {
        when (state) {
            is Test237State.Fail -> {
                // SCE-MAP: test237.scxml:44 :: fail :: _state_body
            }
            is Test237State.Pass -> {
                // SCE-MAP: test237.scxml:43 :: pass :: _state_body
            }
            is Test237State.S0 -> {
                // SCE-MAP: test237.scxml:11 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test237State.S1 -> {
                // SCE-MAP: test237.scxml:34 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test237.scxml:8 :: _machine
    override fun executeTransitionContent(source: Test237State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
