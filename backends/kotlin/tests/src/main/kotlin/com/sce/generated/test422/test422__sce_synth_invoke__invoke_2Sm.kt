// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3 :: _machine

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422SceSynthInvokeInvoke2State : State {
    data object Sub2 : Test422SceSynthInvokeInvoke2State
    data object SubFinal2 : Test422SceSynthInvokeInvoke2State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422SceSynthInvokeInvoke2Event : Event {
    sealed interface Error : Test422SceSynthInvokeInvoke2Event {
        data object Execution : Error
    }
    data object InvokeS12 : Test422SceSynthInvokeInvoke2Event
}
// --- State Machine (W3C SCXML) ---

class Test422SceSynthInvokeInvoke2StateMachine(
) : StateMachineEngine<Test422SceSynthInvokeInvoke2State, Test422SceSynthInvokeInvoke2Event>() {

    override val initialState: Test422SceSynthInvokeInvoke2State = Test422SceSynthInvokeInvoke2State.Sub2

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
    override fun isFinalState(state: Test422SceSynthInvokeInvoke2State): Boolean = when (state) {
        is Test422SceSynthInvokeInvoke2State.SubFinal2 -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test422SceSynthInvokeInvoke2State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test422SceSynthInvokeInvoke2State, HistoryId>> =
            listOf(StateTarget(Test422SceSynthInvokeInvoke2State.Sub2))

        // W3C SCXML 3.13: sub2's transition 0, as the microstep reads it.
        val transitionSub2At0 = EnabledTransition<Test422SceSynthInvokeInvoke2State, HistoryId>(
            Test422SceSynthInvokeInvoke2State.Sub2,
            listOf(StateTarget(Test422SceSynthInvokeInvoke2State.SubFinal2)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422SceSynthInvokeInvoke2State? = when (stateId) {
        "sub2" -> Test422SceSynthInvokeInvoke2State.Sub2
        "subFinal2" -> Test422SceSynthInvokeInvoke2State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422SceSynthInvokeInvoke2State): String = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> "sub2"
        is Test422SceSynthInvokeInvoke2State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test422SceSynthInvokeInvoke2State): Int = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> 0
        is Test422SceSynthInvokeInvoke2State.SubFinal2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422SceSynthInvokeInvoke2Event? = when (name) {
        "error.execution" -> Test422SceSynthInvokeInvoke2Event.Error.Execution
        "invokeS12" -> Test422SceSynthInvokeInvoke2Event.InvokeS12
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422SceSynthInvokeInvoke2Event): String? = when (event) {
        is Test422SceSynthInvokeInvoke2Event.Error.Execution -> "error.execution"
        is Test422SceSynthInvokeInvoke2Event.InvokeS12 -> "invokeS12"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test422SceSynthInvokeInvoke2State,
        event: Test422SceSynthInvokeInvoke2Event?
    ): EnabledTransition<Test422SceSynthInvokeInvoke2State, HistoryId>? = when (state) {
        is Test422SceSynthInvokeInvoke2State.Sub2 -> when {
            event == null -> transitionSub2At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun onEntry(state: Test422SceSynthInvokeInvoke2State, isDefaultEntry: Boolean) {
        when (state) {
            is Test422SceSynthInvokeInvoke2State.Sub2 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:4 :: sub2 :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS12", "")
            }
            is Test422SceSynthInvokeInvoke2State.SubFinal2 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:10 :: subFinal2 :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun onExit(state: Test422SceSynthInvokeInvoke2State) {
        when (state) {
            is Test422SceSynthInvokeInvoke2State.Sub2 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:4 :: sub2 :: _state_body
            }
            is Test422SceSynthInvokeInvoke2State.SubFinal2 -> {
                // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:10 :: subFinal2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test422__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test422SceSynthInvokeInvoke2State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
