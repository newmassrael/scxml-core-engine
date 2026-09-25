// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/229/test229.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test229.scxml:8 :: _machine

package com.sce.generated.test229

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test229State : State {
    data object Fail : Test229State
    data object Pass : Test229State
    data object S0 : Test229State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test229Event : Event {
    sealed interface Cancel : Test229Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test229Event
    sealed interface Done : Test229Event {
        data object Invoke : Done
    }
    sealed interface Error : Test229Event {
        data object Execution : Error
    }
    data object EventReceived : Test229Event
    data object Timeout : Test229Event
}
// --- State Machine (W3C SCXML) ---

class Test229StateMachine(
) : StateMachineEngine<Test229State, Test229Event>() {

    override val initialState: Test229State = Test229State.S0

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
    override fun isFinalState(state: Test229State): Boolean = when (state) {
        is Test229State.Fail, is Test229State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test229State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test229State, HistoryId>> =
            listOf(StateTarget(Test229State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test229State, HistoryId>(
            Test229State.S0,
            emptyList(),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test229State, HistoryId>(
            Test229State.S0,
            listOf(StateTarget(Test229State.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test229State, HistoryId>(
            Test229State.S0,
            listOf(StateTarget(Test229State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test229State? = when (stateId) {
        "fail" -> Test229State.Fail
        "pass" -> Test229State.Pass
        "s0" -> Test229State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test229State): String = when (state) {
        is Test229State.Fail -> "fail"
        is Test229State.Pass -> "pass"
        is Test229State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test229State): Int = when (state) {
        is Test229State.Fail -> 2
        is Test229State.Pass -> 1
        is Test229State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test229Event? = when (name) {
        "cancel.invoke" -> Test229Event.Cancel.Invoke
        "childToParent" -> Test229Event.ChildToParent
        "done.invoke" -> Test229Event.Done.Invoke
        "error.execution" -> Test229Event.Error.Execution
        "eventReceived" -> Test229Event.EventReceived
        "timeout" -> Test229Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test229Event): String? = when (event) {
        is Test229Event.Cancel.Invoke -> "cancel.invoke"
        is Test229Event.ChildToParent -> "childToParent"
        is Test229Event.Done.Invoke -> "done.invoke"
        is Test229Event.Error.Execution -> "error.execution"
        is Test229Event.EventReceived -> "eventReceived"
        is Test229Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test229State,
        event: Test229Event?
    ): EnabledTransition<Test229State, HistoryId>? = when (state) {
        is Test229State.S0 -> when {
            event is Test229Event.ChildToParent -> transitionS0At0
            event is Test229Event.EventReceived -> transitionS0At1
            event != null -> transitionS0At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test229.scxml:8 :: _machine
    override fun onEntry(state: Test229State, isDefaultEntry: Boolean) {
        when (state) {
            is Test229State.Fail -> {
                // SCE-MAP: test229.scxml:44 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test229State.Pass -> {
                // SCE-MAP: test229.scxml:43 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test229State.S0 -> {
                // SCE-MAP: test229.scxml:11 :: s0 :: _state_body


            scheduleSend("__send_0", 3000L, Test229Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test229SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, true, Test229Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test229.scxml:8 :: _machine
    override fun onExit(state: Test229State) {
        when (state) {
            is Test229State.Fail -> {
                // SCE-MAP: test229.scxml:44 :: fail :: _state_body
            }
            is Test229State.Pass -> {
                // SCE-MAP: test229.scxml:43 :: pass :: _state_body
            }
            is Test229State.S0 -> {
                // SCE-MAP: test229.scxml:11 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test229.scxml:8 :: _machine
    override fun executeTransitionContent(source: Test229State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
