// SCE-GENERATED — DO NOT EDIT
// source-hash: 15abee63eca48c0d096ade54003293e94f23f9dffeaf437e4cf29a0ed73c4eb2

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/send_param_payload/send_param_payload__sce_synth_invoke__inv_emitter.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine

package com.sce.integration.send_param_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SendParamPayloadSceSynthInvokeInvEmitterState : State {
    data object Emit : SendParamPayloadSceSynthInvokeInvEmitterState
    data object Sent : SendParamPayloadSceSynthInvokeInvEmitterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SendParamPayloadSceSynthInvokeInvEmitterEvent : Event {
    sealed interface Error : SendParamPayloadSceSynthInvokeInvEmitterEvent {
        data object Execution : Error
    }
    data object FromChild : SendParamPayloadSceSynthInvokeInvEmitterEvent
}
// --- State Machine (W3C SCXML) ---

class SendParamPayloadSceSynthInvokeInvEmitterStateMachine(
) : StateMachineEngine<SendParamPayloadSceSynthInvokeInvEmitterState, SendParamPayloadSceSynthInvokeInvEmitterEvent>() {

    override val initialState: SendParamPayloadSceSynthInvokeInvEmitterState = SendParamPayloadSceSynthInvokeInvEmitterState.Emit

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
    override fun isFinalState(state: SendParamPayloadSceSynthInvokeInvEmitterState): Boolean = when (state) {
        is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<SendParamPayloadSceSynthInvokeInvEmitterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<SendParamPayloadSceSynthInvokeInvEmitterState, HistoryId>> =
            listOf(StateTarget(SendParamPayloadSceSynthInvokeInvEmitterState.Emit))

        // W3C SCXML 3.13: emit's transition 0, as the microstep reads it.
        val transitionEmitAt0 = EnabledTransition<SendParamPayloadSceSynthInvokeInvEmitterState, HistoryId>(
            SendParamPayloadSceSynthInvokeInvEmitterState.Emit,
            listOf(StateTarget(SendParamPayloadSceSynthInvokeInvEmitterState.Sent)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SendParamPayloadSceSynthInvokeInvEmitterState? = when (stateId) {
        "emit" -> SendParamPayloadSceSynthInvokeInvEmitterState.Emit
        "sent" -> SendParamPayloadSceSynthInvokeInvEmitterState.Sent
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SendParamPayloadSceSynthInvokeInvEmitterState): String = when (state) {
        is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> "emit"
        is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> "sent"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: SendParamPayloadSceSynthInvokeInvEmitterState): Int = when (state) {
        is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> 0
        is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SendParamPayloadSceSynthInvokeInvEmitterEvent? = when (name) {
        "error.execution" -> SendParamPayloadSceSynthInvokeInvEmitterEvent.Error.Execution
        "fromChild" -> SendParamPayloadSceSynthInvokeInvEmitterEvent.FromChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SendParamPayloadSceSynthInvokeInvEmitterEvent): String? = when (event) {
        is SendParamPayloadSceSynthInvokeInvEmitterEvent.Error.Execution -> "error.execution"
        is SendParamPayloadSceSynthInvokeInvEmitterEvent.FromChild -> "fromChild"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: SendParamPayloadSceSynthInvokeInvEmitterState,
        event: SendParamPayloadSceSynthInvokeInvEmitterEvent?
    ): EnabledTransition<SendParamPayloadSceSynthInvokeInvEmitterState, HistoryId>? = when (state) {
        is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> when {
            event == null -> transitionEmitAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine
    override fun onEntry(state: SendParamPayloadSceSynthInvokeInvEmitterState, isDefaultEntry: Boolean) {
        when (state) {
            is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:5 :: emit :: _state_body


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                val paramsP = mutableMapOf<String, Any?>()
                putParam(paramsP, "value", "42")
                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("fromChild", eventDataP)
            }
            }
            is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:13 :: sent :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine
    override fun onExit(state: SendParamPayloadSceSynthInvokeInvEmitterState) {
        when (state) {
            is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:5 :: emit :: _state_body
            }
            is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:13 :: sent :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine
    override fun executeTransitionContent(source: SendParamPayloadSceSynthInvokeInvEmitterState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
