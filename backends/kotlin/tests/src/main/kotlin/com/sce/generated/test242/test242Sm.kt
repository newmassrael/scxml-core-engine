// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242.scxml:6 :: _machine

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242State : State {
    data object Fail : Test242State
    data object Pass : Test242State
    data object S0 : Test242State
    data object S02 : Test242State
    data object S03 : Test242State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242Event : Event {
    sealed interface Done : Test242Event {
        data object Invoke : Done
    }
    sealed interface Error : Test242Event {
        data object Execution : Error
    }
    data object Timeout : Test242Event
    data object Timeout1 : Test242Event
    data object Timeout2 : Test242Event
    data object Timeout3 : Test242Event
}
// --- State Machine (W3C SCXML) ---

class Test242StateMachine(
) : StateMachineEngine<Test242State, Test242Event>() {

    override val initialState: Test242State = Test242State.S0

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
    override fun isFinalState(state: Test242State): Boolean = when (state) {
        is Test242State.Fail, is Test242State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test242State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test242State, HistoryId>> =
            listOf(StateTarget(Test242State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.S02)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.S03)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S02,
            listOf(StateTarget(Test242State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S02,
            listOf(StateTarget(Test242State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S03,
            listOf(StateTarget(Test242State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S03,
            listOf(StateTarget(Test242State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242State? = when (stateId) {
        "fail" -> Test242State.Fail
        "pass" -> Test242State.Pass
        "s0" -> Test242State.S0
        "s02" -> Test242State.S02
        "s03" -> Test242State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242State): String = when (state) {
        is Test242State.Fail -> "fail"
        is Test242State.Pass -> "pass"
        is Test242State.S0 -> "s0"
        is Test242State.S02 -> "s02"
        is Test242State.S03 -> "s03"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test242State): Int = when (state) {
        is Test242State.Fail -> 4
        is Test242State.Pass -> 3
        is Test242State.S0 -> 0
        is Test242State.S02 -> 1
        is Test242State.S03 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test242Event? = when (name) {
        "done.invoke" -> Test242Event.Done.Invoke
        "error.execution" -> Test242Event.Error.Execution
        "timeout" -> Test242Event.Timeout
        "timeout1" -> Test242Event.Timeout1
        "timeout2" -> Test242Event.Timeout2
        "timeout3" -> Test242Event.Timeout3
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test242Event): String? = when (event) {
        is Test242Event.Done.Invoke -> "done.invoke"
        is Test242Event.Error.Execution -> "error.execution"
        is Test242Event.Timeout -> "timeout"
        is Test242Event.Timeout1 -> "timeout1"
        is Test242Event.Timeout2 -> "timeout2"
        is Test242Event.Timeout3 -> "timeout3"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test242State,
        event: Test242Event?
    ): EnabledTransition<Test242State, HistoryId>? = when (state) {
        is Test242State.S0 -> when {
            event is Test242Event.Timeout -> transitionS0At0
            event is Test242Event.Done.Invoke -> transitionS0At1
            event is Test242Event.Timeout1 -> transitionS0At2
            else -> null
        }
        is Test242State.S02 -> when {
            event is Test242Event.Done.Invoke -> transitionS02At0
            event is Test242Event.Timeout2 -> transitionS02At1
            else -> null
        }
        is Test242State.S03 -> when {
            event is Test242Event.Timeout3 -> transitionS03At0
            event is Test242Event.Done.Invoke -> transitionS03At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun onEntry(state: Test242State, isDefaultEntry: Boolean) {
        when (state) {
            is Test242State.Fail -> {
                // SCE-MAP: test242.scxml:56 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.Pass -> {
                // SCE-MAP: test242.scxml:55 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.S0 -> {
                // SCE-MAP: test242.scxml:9 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test242Event.Timeout1)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242sub1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test242State.S02 -> {
                // SCE-MAP: test242.scxml:20 :: s02 :: _state_body


            scheduleSend("__send_1", 1000L, Test242Event.Timeout2)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242SceSynthInvokeInvoke1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test242State.S03 -> {
                // SCE-MAP: test242.scxml:37 :: s03 :: _state_body


            scheduleSend("__send_2", 1000L, Test242Event.Timeout3)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s03.${System.identityHashCode(this)}._invoke_2"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242SceSynthInvokeInvoke2StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_2", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun onExit(state: Test242State) {
        when (state) {
            is Test242State.Fail -> {
                // SCE-MAP: test242.scxml:56 :: fail :: _state_body
            }
            is Test242State.Pass -> {
                // SCE-MAP: test242.scxml:55 :: pass :: _state_body
            }
            is Test242State.S0 -> {
                // SCE-MAP: test242.scxml:9 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test242State.S02 -> {
                // SCE-MAP: test242.scxml:20 :: s02 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
            }
            is Test242State.S03 -> {
                // SCE-MAP: test242.scxml:37 :: s03 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test242State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
