// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine

package com.sce.generated.test234

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test234SceSynthInvokeInvoke1State : State {
    data object Sub0 : Test234SceSynthInvokeInvoke1State
    data object SubFinal2 : Test234SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test234SceSynthInvokeInvoke1Event : Event {
    sealed interface Error : Test234SceSynthInvokeInvoke1Event {
        data object Execution : Error
    }
    data object Timeout : Test234SceSynthInvokeInvoke1Event
}
// --- State Machine (W3C SCXML) ---

class Test234SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test234SceSynthInvokeInvoke1State, Test234SceSynthInvokeInvoke1Event>() {

    override val initialState: Test234SceSynthInvokeInvoke1State = Test234SceSynthInvokeInvoke1State.Sub0

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
    override fun isFinalState(state: Test234SceSynthInvokeInvoke1State): Boolean = when (state) {
        is Test234SceSynthInvokeInvoke1State.SubFinal2 -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test234SceSynthInvokeInvoke1State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test234SceSynthInvokeInvoke1State, HistoryId>> =
            listOf(StateTarget(Test234SceSynthInvokeInvoke1State.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test234SceSynthInvokeInvoke1State, HistoryId>(
            Test234SceSynthInvokeInvoke1State.Sub0,
            listOf(StateTarget(Test234SceSynthInvokeInvoke1State.SubFinal2)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test234SceSynthInvokeInvoke1State? = when (stateId) {
        "sub0" -> Test234SceSynthInvokeInvoke1State.Sub0
        "subFinal2" -> Test234SceSynthInvokeInvoke1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test234SceSynthInvokeInvoke1State): String = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> "sub0"
        is Test234SceSynthInvokeInvoke1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test234SceSynthInvokeInvoke1State): Int = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> 0
        is Test234SceSynthInvokeInvoke1State.SubFinal2 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test234SceSynthInvokeInvoke1State,
        event: Test234SceSynthInvokeInvoke1Event?
    ): EnabledTransition<Test234SceSynthInvokeInvoke1State, HistoryId>? = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> when {
            event is Test234SceSynthInvokeInvoke1Event.Timeout -> transitionSub0At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onEntry(state: Test234SceSynthInvokeInvoke1State, isDefaultEntry: Boolean) {
        when (state) {
            is Test234SceSynthInvokeInvoke1State.Sub0 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:4 :: sub0 :: _state_body


            scheduleSend("__send_0", 2000L, Test234SceSynthInvokeInvoke1Event.Timeout)
            }
            is Test234SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:10 :: subFinal2 :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onExit(state: Test234SceSynthInvokeInvoke1State) {
        when (state) {
            is Test234SceSynthInvokeInvoke1State.Sub0 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:4 :: sub0 :: _state_body
            }
            is Test234SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:10 :: subFinal2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test234SceSynthInvokeInvoke1State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
