// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test225

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test225SceSynthInvokeInvoke0State : State {
    data object SubFinal1 : Test225SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test225SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test225SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test225SceSynthInvokeInvoke0State, Test225SceSynthInvokeInvoke0Event>() {

    override val initialState: Test225SceSynthInvokeInvoke0State = Test225SceSynthInvokeInvoke0State.SubFinal1

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
    override fun isFinalState(state: Test225SceSynthInvokeInvoke0State): Boolean = when (state) {
        is Test225SceSynthInvokeInvoke0State.SubFinal1 -> true
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test225SceSynthInvokeInvoke0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test225SceSynthInvokeInvoke0State, HistoryId>> =
            listOf(StateTarget(Test225SceSynthInvokeInvoke0State.SubFinal1))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test225SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal1" -> Test225SceSynthInvokeInvoke0State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test225SceSynthInvokeInvoke0State): String = when (state) {
        is Test225SceSynthInvokeInvoke0State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test225SceSynthInvokeInvoke0State): Int = when (state) {
        is Test225SceSynthInvokeInvoke0State.SubFinal1 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test225SceSynthInvokeInvoke0State,
        event: Test225SceSynthInvokeInvoke0Event?
    ): EnabledTransition<Test225SceSynthInvokeInvoke0State, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test225SceSynthInvokeInvoke0State, isDefaultEntry: Boolean) {
        when (state) {
            is Test225SceSynthInvokeInvoke0State.SubFinal1 -> {
                // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:4 :: subFinal1 :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test225SceSynthInvokeInvoke0State) {
        when (state) {
            is Test225SceSynthInvokeInvoke0State.SubFinal1 -> {
                // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:4 :: subFinal1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test225SceSynthInvokeInvoke0State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
