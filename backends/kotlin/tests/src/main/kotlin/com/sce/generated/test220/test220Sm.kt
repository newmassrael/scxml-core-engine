// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/220/test220.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test220.scxml:5 :: _machine

package com.sce.generated.test220

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test220State : State {
    data object Fail : Test220State
    data object Pass : Test220State
    data object S0 : Test220State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test220Event : Event {
    sealed interface Cancel : Test220Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test220Event {
        data object Invoke : Done
    }
    sealed interface Error : Test220Event {
        data object Execution : Error
    }
    data object Timeout : Test220Event
}
// --- State Machine (W3C SCXML) ---

class Test220StateMachine(
) : StateMachineEngine<Test220State, Test220Event>() {

    override val initialState: Test220State = Test220State.S0

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
    override fun isFinalState(state: Test220State): Boolean = when (state) {
        is Test220State.Fail, is Test220State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test220State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test220State, HistoryId>> =
            listOf(StateTarget(Test220State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test220State, HistoryId>(
            Test220State.S0,
            listOf(StateTarget(Test220State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test220State, HistoryId>(
            Test220State.S0,
            listOf(StateTarget(Test220State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test220State? = when (stateId) {
        "fail" -> Test220State.Fail
        "pass" -> Test220State.Pass
        "s0" -> Test220State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test220State): String = when (state) {
        is Test220State.Fail -> "fail"
        is Test220State.Pass -> "pass"
        is Test220State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test220State): Int = when (state) {
        is Test220State.Fail -> 2
        is Test220State.Pass -> 1
        is Test220State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test220Event? = when (name) {
        "cancel.invoke" -> Test220Event.Cancel.Invoke
        "done.invoke" -> Test220Event.Done.Invoke
        "error.execution" -> Test220Event.Error.Execution
        "timeout" -> Test220Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test220Event): String? = when (event) {
        is Test220Event.Cancel.Invoke -> "cancel.invoke"
        is Test220Event.Done.Invoke -> "done.invoke"
        is Test220Event.Error.Execution -> "error.execution"
        is Test220Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test220State,
        event: Test220Event?
    ): EnabledTransition<Test220State, HistoryId>? = when (state) {
        is Test220State.S0 -> when {
            event is Test220Event.Done.Invoke -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test220.scxml:5 :: _machine
    override fun onEntry(state: Test220State, isDefaultEntry: Boolean) {
        when (state) {
            is Test220State.Fail -> {
                // SCE-MAP: test220.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.Pass -> {
                // SCE-MAP: test220.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.S0 -> {
                // SCE-MAP: test220.scxml:7 :: s0 :: _state_body


            scheduleSend("__send_0", 5000L, Test220Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test220SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test220Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test220.scxml:5 :: _machine
    override fun onExit(state: Test220State) {
        when (state) {
            is Test220State.Fail -> {
                // SCE-MAP: test220.scxml:23 :: fail :: _state_body
            }
            is Test220State.Pass -> {
                // SCE-MAP: test220.scxml:22 :: pass :: _state_body
            }
            is Test220State.S0 -> {
                // SCE-MAP: test220.scxml:7 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test220.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test220State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
