// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test338

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test338SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test338SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test338SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test338SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Event1 : Test338SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test338SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test338SceSynthInvokeInvoke0State, Test338SceSynthInvokeInvoke0Event>() {

    override val initialState: Test338SceSynthInvokeInvoke0State = Test338SceSynthInvokeInvoke0State.Sub0

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
    override fun isFinalState(state: Test338SceSynthInvokeInvoke0State): Boolean = when (state) {
        is Test338SceSynthInvokeInvoke0State.Sub0 -> true
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test338SceSynthInvokeInvoke0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test338SceSynthInvokeInvoke0State, HistoryId>> =
            listOf(StateTarget(Test338SceSynthInvokeInvoke0State.Sub0))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test338SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test338SceSynthInvokeInvoke0State.Sub0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test338SceSynthInvokeInvoke0State): String = when (state) {
        is Test338SceSynthInvokeInvoke0State.Sub0 -> "sub0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test338SceSynthInvokeInvoke0State): Int = when (state) {
        is Test338SceSynthInvokeInvoke0State.Sub0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test338SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test338SceSynthInvokeInvoke0Event.Error.Execution
        "event1" -> Test338SceSynthInvokeInvoke0Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test338SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test338SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test338SceSynthInvokeInvoke0Event.Event1 -> "event1"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test338SceSynthInvokeInvoke0State,
        event: Test338SceSynthInvokeInvoke0Event?
    ): EnabledTransition<Test338SceSynthInvokeInvoke0State, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test338SceSynthInvokeInvoke0State, isDefaultEntry: Boolean) {
        when (state) {
            is Test338SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("event1", "")
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test338SceSynthInvokeInvoke0State) {
        when (state) {
            is Test338SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test338SceSynthInvokeInvoke0State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
