// SCE-GENERATED — DO NOT EDIT
// source-hash: 34a3aa3a202a7ed359ee0ab4d1bade13a9630715ad1b32496ec4d19b29db3f4f
// template-hash: ebaa86fbc385aba6e768cc24b7caf9cb286a422578625ecd77b79d34c33f2e74
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_candidate_selects_the_child/other.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: other.scxml:6 :: _machine

package com.sce.integration.invoke_candidate_selects_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface OtherState : State {
    data object Done : OtherState
    data object Speak : OtherState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface OtherEvent : Event {
    sealed interface Error : OtherEvent {
        data object Execution : Error
    }
    sealed interface From : OtherEvent {
        data object Other : From
    }
}
// --- State Machine (W3C SCXML) ---

class OtherStateMachine(
) : StateMachineEngine<OtherState, OtherEvent>() {

    override val initialState: OtherState = OtherState.Speak

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): OtherState? = when (stateId) {
        "done" -> OtherState.Done
        "speak" -> OtherState.Speak
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: OtherState): String = when (state) {
        is OtherState.Done -> "done"
        is OtherState.Speak -> "speak"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: OtherState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: OtherState): Int = when (state) {
        is OtherState.Done -> 1
        is OtherState.Speak -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): OtherEvent? = when (name) {
        "error.execution" -> OtherEvent.Error.Execution
        "from.other" -> OtherEvent.From.Other
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: OtherEvent): String? = when (event) {
        is OtherEvent.Error.Execution -> "error.execution"
        is OtherEvent.From.Other -> "from.other"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: OtherState,
        event: OtherEvent
    ): TransitionResult<OtherState> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: OtherState
    ): TransitionResult<OtherState> = when (state) {
        is OtherState.Speak -> processNullSpeak()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSpeak(
    ): TransitionResult<OtherState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(OtherState.Done, OtherState.Speak, 0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: other.scxml:6 :: _machine
    override fun onEntry(state: OtherState, pathChild: OtherState?) {
        when (state) {
            is OtherState.Done -> {
                // SCE-MAP: other.scxml:14 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is OtherState.Speak -> {
                // SCE-MAP: other.scxml:8 :: speak :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("speak")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("from.other", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: other.scxml:6 :: _machine
    override fun onExit(state: OtherState) {
        when (state) {
            is OtherState.Done -> {
                // SCE-MAP: other.scxml:14 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is OtherState.Speak -> {
                // SCE-MAP: other.scxml:8 :: speak :: _state_body
                activeStateIds.remove("speak")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: other.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: OtherState,
        event: OtherEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
