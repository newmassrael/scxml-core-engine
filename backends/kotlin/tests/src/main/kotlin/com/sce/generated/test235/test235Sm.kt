// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/235/test235.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test235.scxml:6 :: _machine

package com.sce.generated.test235

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test235State : State {
    data object Fail : Test235State
    data object Pass : Test235State
    data object S0 : Test235State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test235Event : Event {
    sealed interface Done : Test235Event {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Foo : Invoke
        }
    }
    sealed interface Error : Test235Event {
        data object Execution : Error
    }
    data object Timeout : Test235Event
}
// --- State Machine (W3C SCXML) ---

class Test235StateMachine(
) : StateMachineEngine<Test235State, Test235Event>() {

    override val initialState: Test235State = Test235State.S0

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
    override fun isFinalState(state: Test235State): Boolean = when (state) {
        is Test235State.Fail, is Test235State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test235State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test235State, HistoryId>> =
            listOf(StateTarget(Test235State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test235State, HistoryId>(
            Test235State.S0,
            listOf(StateTarget(Test235State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test235State, HistoryId>(
            Test235State.S0,
            listOf(StateTarget(Test235State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test235State? = when (stateId) {
        "fail" -> Test235State.Fail
        "pass" -> Test235State.Pass
        "s0" -> Test235State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test235State): String = when (state) {
        is Test235State.Fail -> "fail"
        is Test235State.Pass -> "pass"
        is Test235State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test235State): Int = when (state) {
        is Test235State.Fail -> 2
        is Test235State.Pass -> 1
        is Test235State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test235Event? = when (name) {
        "done.invoke" -> Test235Event.Done.Invoke.Self
        "done.invoke.foo" -> Test235Event.Done.Invoke.Foo
        "error.execution" -> Test235Event.Error.Execution
        "timeout" -> Test235Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test235Event): String? = when (event) {
        is Test235Event.Done.Invoke.Self -> "done.invoke"
        is Test235Event.Done.Invoke.Foo -> "done.invoke.foo"
        is Test235Event.Error.Execution -> "error.execution"
        is Test235Event.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test235State,
        event: Test235Event?
    ): EnabledTransition<Test235State, HistoryId>? = when (state) {
        is Test235State.S0 -> when {
            event is Test235Event.Done.Invoke.Foo -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test235.scxml:6 :: _machine
    override fun onEntry(state: Test235State, isDefaultEntry: Boolean) {
        when (state) {
            is Test235State.Fail -> {
                // SCE-MAP: test235.scxml:26 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.Pass -> {
                // SCE-MAP: test235.scxml:25 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.S0 -> {
                // SCE-MAP: test235.scxml:9 :: s0 :: _state_body


            scheduleSend("__send_0", 2000L, Test235Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.foo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test235SceSynthInvokeFooStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("foo", childSM, false, Test235Event.Done.Invoke.Foo, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test235.scxml:6 :: _machine
    override fun onExit(state: Test235State) {
        when (state) {
            is Test235State.Fail -> {
                // SCE-MAP: test235.scxml:26 :: fail :: _state_body
            }
            is Test235State.Pass -> {
                // SCE-MAP: test235.scxml:25 :: pass :: _state_body
            }
            is Test235State.S0 -> {
                // SCE-MAP: test235.scxml:9 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("foo")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test235.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test235State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
