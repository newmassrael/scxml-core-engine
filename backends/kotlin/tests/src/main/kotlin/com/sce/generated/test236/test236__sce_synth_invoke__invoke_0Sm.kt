// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test236

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test236SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test236SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test236SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test236SceSynthInvokeInvoke0Event
    sealed interface Error : Test236SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test236SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test236SceSynthInvokeInvoke0State, Test236SceSynthInvokeInvoke0Event>() {

    override val initialState: Test236SceSynthInvokeInvoke0State = Test236SceSynthInvokeInvoke0State.SubFinal

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
    override fun isFinalState(state: Test236SceSynthInvokeInvoke0State): Boolean = when (state) {
        is Test236SceSynthInvokeInvoke0State.SubFinal -> true
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test236SceSynthInvokeInvoke0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test236SceSynthInvokeInvoke0State, HistoryId>> =
            listOf(StateTarget(Test236SceSynthInvokeInvoke0State.SubFinal))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test236SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test236SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test236SceSynthInvokeInvoke0State): String = when (state) {
        is Test236SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test236SceSynthInvokeInvoke0State): Int = when (state) {
        is Test236SceSynthInvokeInvoke0State.SubFinal -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test236SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test236SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test236SceSynthInvokeInvoke0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test236SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test236SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test236SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test236SceSynthInvokeInvoke0State,
        event: Test236SceSynthInvokeInvoke0Event?
    ): EnabledTransition<Test236SceSynthInvokeInvoke0State, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test236SceSynthInvokeInvoke0State, isDefaultEntry: Boolean) {
        when (state) {
            is Test236SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test236SceSynthInvokeInvoke0State) {
        when (state) {
            is Test236SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test236SceSynthInvokeInvoke0State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
