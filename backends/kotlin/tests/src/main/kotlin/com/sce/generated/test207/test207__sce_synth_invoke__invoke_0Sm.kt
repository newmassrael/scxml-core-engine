// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test207

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test207SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test207SceSynthInvokeInvoke0State
    data object SubFinal : Test207SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test207SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test207SceSynthInvokeInvoke0Event
    sealed interface Error : Test207SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Event1 : Test207SceSynthInvokeInvoke0Event
    data object Event2 : Test207SceSynthInvokeInvoke0Event
    data object Fail : Test207SceSynthInvokeInvoke0Event
    data object Pass : Test207SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test207SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test207SceSynthInvokeInvoke0State, Test207SceSynthInvokeInvoke0Event>() {

    override val initialState: Test207SceSynthInvokeInvoke0State = Test207SceSynthInvokeInvoke0State.Sub0

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
    override fun isFinalState(state: Test207SceSynthInvokeInvoke0State): Boolean = when (state) {
        is Test207SceSynthInvokeInvoke0State.SubFinal -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test207SceSynthInvokeInvoke0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test207SceSynthInvokeInvoke0State, HistoryId>> =
            listOf(StateTarget(Test207SceSynthInvokeInvoke0State.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test207SceSynthInvokeInvoke0State, HistoryId>(
            Test207SceSynthInvokeInvoke0State.Sub0,
            listOf(StateTarget(Test207SceSynthInvokeInvoke0State.SubFinal)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: sub0's transition 1, as the microstep reads it.
        val transitionSub0At1 = EnabledTransition<Test207SceSynthInvokeInvoke0State, HistoryId>(
            Test207SceSynthInvokeInvoke0State.Sub0,
            listOf(StateTarget(Test207SceSynthInvokeInvoke0State.SubFinal)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test207SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test207SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test207SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test207SceSynthInvokeInvoke0State): String = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test207SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test207SceSynthInvokeInvoke0State): Int = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test207SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test207SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test207SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test207SceSynthInvokeInvoke0Event.Error.Execution
        "event1" -> Test207SceSynthInvokeInvoke0Event.Event1
        "event2" -> Test207SceSynthInvokeInvoke0Event.Event2
        "fail" -> Test207SceSynthInvokeInvoke0Event.Fail
        "pass" -> Test207SceSynthInvokeInvoke0Event.Pass
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test207SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test207SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test207SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test207SceSynthInvokeInvoke0Event.Event1 -> "event1"
        is Test207SceSynthInvokeInvoke0Event.Event2 -> "event2"
        is Test207SceSynthInvokeInvoke0Event.Fail -> "fail"
        is Test207SceSynthInvokeInvoke0Event.Pass -> "pass"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test207SceSynthInvokeInvoke0State,
        event: Test207SceSynthInvokeInvoke0Event?
    ): EnabledTransition<Test207SceSynthInvokeInvoke0State, HistoryId>? = when (state) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> when {
            event is Test207SceSynthInvokeInvoke0Event.Event1 -> transitionSub0At0
            event != null -> transitionSub0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test207SceSynthInvokeInvoke0State, isDefaultEntry: Boolean) {
        when (state) {
            is Test207SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body


            scheduleSend("foo", 1000L, Test207SceSynthInvokeInvoke0Event.Event1)


            scheduleSend("__send_2", 1500L, Test207SceSynthInvokeInvoke0Event.Event2)


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test207SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:19 :: subFinal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test207SceSynthInvokeInvoke0State) {
        when (state) {
            is Test207SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
            }
            is Test207SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:19 :: subFinal :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test207SceSynthInvokeInvoke0State, transitionIndex: Int) {
        when (source) {
        is Test207SceSynthInvokeInvoke0State.Sub0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:11 :: sub0 :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("pass", "")
            }
            1 -> {
                // SCE-MAP: test207__sce_synth_invoke__invoke_0.scxml:14 :: sub0 :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("fail", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
