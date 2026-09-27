// SCE-GENERATED — DO NOT EDIT
// source-hash: 61a11c1135bc9b5297b489b5bdeec8ed213e07a866294a88591eded59b857e99

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:3 :: _machine

package com.sce.integration.a_bad_invoke_argument_is_reported_once

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State : State {
    data object Gone : ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State
    data object Up : ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event : Event {
    data object ChildUp : ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event
    sealed interface Error : ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2StateMachine(
) : StateMachineEngine<ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event>() {

    override val initialState: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State = ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up

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
    override fun isFinalState(state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State): Boolean = when (state) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, HistoryId>> =
            listOf(StateTarget(ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up))

        // W3C SCXML 3.13: up's transition 0, as the microstep reads it.
        val transitionUpAt0 = EnabledTransition<ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, HistoryId>(
            ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up,
            listOf(StateTarget(ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State? = when (stateId) {
        "gone" -> ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone
        "up" -> ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State): String = when (state) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone -> "gone"
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> "up"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State): Int = when (state) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone -> 1
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event? = when (name) {
        "childUp" -> ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event.ChildUp
        "error.execution" -> ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event): String? = when (event) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event.ChildUp -> "childUp"
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State,
        event: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2Event?
    ): EnabledTransition<ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, HistoryId>? = when (state) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> when {
            event == null -> transitionUpAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:3 :: _machine
    override fun onEntry(state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, isDefaultEntry: Boolean) {
        when (state) {
            is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:10 :: gone :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:5 :: up :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:3 :: _machine
    override fun onExit(state: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State) {
        when (state) {
            is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Gone -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:10 :: gone :: _state_body
            }
            is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:5 :: up :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:3 :: _machine
    override fun executeTransitionContent(source: ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State, transitionIndex: Int) {
        when (source) {
        is ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2State.Up -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once__sce_synth_invoke__inv2.scxml:6 :: up :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childUp", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
