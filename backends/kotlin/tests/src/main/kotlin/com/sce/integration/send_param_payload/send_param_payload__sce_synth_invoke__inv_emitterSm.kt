// SCE-GENERATED — DO NOT EDIT
// source-hash: f524cef2066d73e88ffccad2cc701b342488a62174d08ce0c0f5b45b36ac885f
// template-hash: 60da764009afb96185d876c542254f2e8363dba627394829757a2a8f121eddd1
// generated-at: 0

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SendParamPayloadSceSynthInvokeInvEmitterState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: SendParamPayloadSceSynthInvokeInvEmitterState,
        event: SendParamPayloadSceSynthInvokeInvEmitterEvent
    ): TransitionResult<SendParamPayloadSceSynthInvokeInvEmitterState> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: SendParamPayloadSceSynthInvokeInvEmitterState
    ): TransitionResult<SendParamPayloadSceSynthInvokeInvEmitterState> = when (state) {
        is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> processNullEmit()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullEmit(
    ): TransitionResult<SendParamPayloadSceSynthInvokeInvEmitterState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(SendParamPayloadSceSynthInvokeInvEmitterState.Sent, SendParamPayloadSceSynthInvokeInvEmitterState.Emit)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine
    override fun onEntry(state: SendParamPayloadSceSynthInvokeInvEmitterState, pathChild: SendParamPayloadSceSynthInvokeInvEmitterState?) {
        when (state) {
            is SendParamPayloadSceSynthInvokeInvEmitterState.Emit -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:5 :: emit :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("emit")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sent")) return
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
                activeStateIds.remove("emit")
            }
            is SendParamPayloadSceSynthInvokeInvEmitterState.Sent -> {
                // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:13 :: sent :: _state_body
                activeStateIds.remove("sent")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: send_param_payload__sce_synth_invoke__inv_emitter.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: SendParamPayloadSceSynthInvokeInvEmitterState,
        event: SendParamPayloadSceSynthInvokeInvEmitterEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
