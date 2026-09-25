// SCE-GENERATED — DO NOT EDIT
// source-hash: e67e22f50324628b768bd45c270ec785da7ac8d8eb5d881012137ffe720d345e

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InvokeUnsupportedTypeState): Boolean = when (state) {
        is InvokeUnsupportedTypeState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokeUnsupportedTypeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokeUnsupportedTypeState, HistoryId>> =
            listOf(StateTarget(InvokeUnsupportedTypeState.Probe))

        // W3C SCXML 3.13: probe's transition 0, as the microstep reads it.
        val transitionProbeAt0 = EnabledTransition<InvokeUnsupportedTypeState, HistoryId>(
            InvokeUnsupportedTypeState.Probe,
            listOf(StateTarget(InvokeUnsupportedTypeState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: InvokeUnsupportedTypeState,
        event: InvokeUnsupportedTypeEvent?
    ): EnabledTransition<InvokeUnsupportedTypeState, HistoryId>? = when (state) {
        is InvokeUnsupportedTypeState.Probe -> when {
            event is InvokeUnsupportedTypeEvent.Error.Execution -> transitionProbeAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine
    override fun onEntry(state: InvokeUnsupportedTypeState, isDefaultEntry: Boolean) {
        when (state) {
            is InvokeUnsupportedTypeState.Pass -> {
                // SCE-MAP: invoke_unsupported_type.scxml:42 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeUnsupportedTypeState.Probe -> {
                // SCE-MAP: invoke_unsupported_type.scxml:38 :: probe :: _state_body
                // W3C SCXML 6.4.1: `type` names no processor this platform
                // implements. The deferred closure runs at macrostep end and
                // raises error.execution — no child is created, so nothing
                // follows and state exit has nothing to cancel beyond the
                // pending entry `cancelPendingInvokesForState` already drops.
                run {
                    val generatedInvokeId = "probe.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        raisePlatformError(InvokeUnsupportedTypeEvent.Error.Execution, "<invoke type='urn:example:no-such-processor'> names an external service this platform does not support")
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
            }
            is InvokeUnsupportedTypeState.Probe -> {
                // SCE-MAP: invoke_unsupported_type.scxml:38 :: probe :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_unsupported_type.scxml:35 :: _machine
    override fun executeTransitionContent(source: InvokeUnsupportedTypeState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
