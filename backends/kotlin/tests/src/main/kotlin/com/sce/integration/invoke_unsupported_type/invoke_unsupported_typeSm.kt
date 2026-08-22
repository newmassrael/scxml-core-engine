// SCE-GENERATED — DO NOT EDIT
// source-hash: e67e22f50324628b768bd45c270ec785da7ac8d8eb5d881012137ffe720d345e
// template-hash: 465642caa5c7ae5f006b7e4c3302ebaf26878f27c380322c3cf9d87ca35b0ee6
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine

package com.sce.integration.invoke_unsupported_type

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeUnsupportedTypeState : State {
    data object Pass : InvokeUnsupportedTypeState
    data object Probe : InvokeUnsupportedTypeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeUnsupportedTypeEvent : Event {
    sealed interface Error : InvokeUnsupportedTypeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeUnsupportedTypeStateMachine(
) : StateMachineEngine<InvokeUnsupportedTypeState, InvokeUnsupportedTypeEvent>() {

    override val initialState: InvokeUnsupportedTypeState = InvokeUnsupportedTypeState.Probe

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokeUnsupportedTypeState? = when (stateId) {
        "pass" -> InvokeUnsupportedTypeState.Pass
        "probe" -> InvokeUnsupportedTypeState.Probe
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeUnsupportedTypeState): String = when (state) {
        is InvokeUnsupportedTypeState.Pass -> "pass"
        is InvokeUnsupportedTypeState.Probe -> "probe"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeUnsupportedTypeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeUnsupportedTypeState): Int = when (state) {
        is InvokeUnsupportedTypeState.Pass -> 1
        is InvokeUnsupportedTypeState.Probe -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeUnsupportedTypeEvent? = when (name) {
        "error.execution" -> InvokeUnsupportedTypeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeUnsupportedTypeEvent): String? = when (event) {
        is InvokeUnsupportedTypeEvent.Error.Execution -> "error.execution"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokeUnsupportedTypeState,
        event: InvokeUnsupportedTypeEvent
    ): TransitionResult<InvokeUnsupportedTypeState> = when (state) {
        is InvokeUnsupportedTypeState.Probe -> processProbe(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processProbe(
        event: InvokeUnsupportedTypeEvent
    ): TransitionResult<InvokeUnsupportedTypeState> = when {
        event is InvokeUnsupportedTypeEvent.Error.Execution -> TransitionResult.External(InvokeUnsupportedTypeState.Pass, InvokeUnsupportedTypeState.Probe)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine
    override fun onEntry(state: InvokeUnsupportedTypeState, pathChild: InvokeUnsupportedTypeState?) {
        when (state) {
            is InvokeUnsupportedTypeState.Pass -> {
                // SCE-MAP: invoke_unsupported_type.scxml:42 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeUnsupportedTypeState.Probe -> {
                // SCE-MAP: invoke_unsupported_type.scxml:38 :: probe :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("probe")) return
                // W3C SCXML 6.4.1: `type` names no processor this platform
                // implements. The deferred closure runs at macrostep end and
                // raises error.execution — no child is created, so nothing
                // follows and state exit has nothing to cancel beyond the
                // pending entry `cancelPendingInvokesForState` already drops.
                run {
                    val generatedInvokeId = "probe.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        raisePlatformError(InvokeUnsupportedTypeEvent.Error.Execution, "<invoke> declares a type this processor does not support")
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine
    override fun onExit(state: InvokeUnsupportedTypeState) {
        when (state) {
            is InvokeUnsupportedTypeState.Pass -> {
                // SCE-MAP: invoke_unsupported_type.scxml:42 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokeUnsupportedTypeState.Probe -> {
                // SCE-MAP: invoke_unsupported_type.scxml:38 :: probe :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                activeStateIds.remove("probe")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine
    override fun executeTransitionActions(
        source: InvokeUnsupportedTypeState,
        event: InvokeUnsupportedTypeEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
