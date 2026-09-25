// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test422SceSynthInvokeInvoke0State
    data object SubFinal0 : Test422SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test422SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object InvokeS1 : Test422SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test422SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test422SceSynthInvokeInvoke0State, Test422SceSynthInvokeInvoke0Event>() {

    override val initialState: Test422SceSynthInvokeInvoke0State = Test422SceSynthInvokeInvoke0State.Sub0

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
    override fun isFinalState(state: Test422SceSynthInvokeInvoke0State): Boolean = when (state) {
        is Test422SceSynthInvokeInvoke0State.SubFinal0 -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test422SceSynthInvokeInvoke0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test422SceSynthInvokeInvoke0State, HistoryId>> =
            listOf(StateTarget(Test422SceSynthInvokeInvoke0State.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test422SceSynthInvokeInvoke0State, HistoryId>(
            Test422SceSynthInvokeInvoke0State.Sub0,
            listOf(StateTarget(Test422SceSynthInvokeInvoke0State.SubFinal0)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test422SceSynthInvokeInvoke0State.Sub0
        "subFinal0" -> Test422SceSynthInvokeInvoke0State.SubFinal0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422SceSynthInvokeInvoke0State): String = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test422SceSynthInvokeInvoke0State.SubFinal0 -> "subFinal0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test422SceSynthInvokeInvoke0State): Int = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test422SceSynthInvokeInvoke0State.SubFinal0 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test422SceSynthInvokeInvoke0Event.Error.Execution
        "invokeS1" -> Test422SceSynthInvokeInvoke0Event.InvokeS1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test422SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test422SceSynthInvokeInvoke0Event.InvokeS1 -> "invokeS1"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test422SceSynthInvokeInvoke0State,
        event: Test422SceSynthInvokeInvoke0Event?
    ): EnabledTransition<Test422SceSynthInvokeInvoke0State, HistoryId>? = when (state) {
        is Test422SceSynthInvokeInvoke0State.Sub0 -> when {
            event == null -> transitionSub0At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test422SceSynthInvokeInvoke0State, isDefaultEntry: Boolean) {
        when (state) {
            is Test422SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS1", "")
            }
            is Test422SceSynthInvokeInvoke0State.SubFinal0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:10 :: subFinal0 :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test422SceSynthInvokeInvoke0State) {
        when (state) {
            is Test422SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
            }
            is Test422SceSynthInvokeInvoke0State.SubFinal0 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:10 :: subFinal0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test422__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test422SceSynthInvokeInvoke0State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
