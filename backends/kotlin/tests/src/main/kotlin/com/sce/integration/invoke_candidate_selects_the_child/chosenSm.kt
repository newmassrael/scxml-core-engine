// SCE-GENERATED — DO NOT EDIT
// source-hash: 34a3aa3a202a7ed359ee0ab4d1bade13a9630715ad1b32496ec4d19b29db3f4f
// template-hash: 68cd6517eb7ab30f12195ed5715cf739261174462172b0a4bc940ea551e29052
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_candidate_selects_the_child/chosen.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: chosen.scxml:5 :: _machine

package com.sce.integration.invoke_candidate_selects_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ChosenState : State {
    data object Done : ChosenState
    data object Speak : ChosenState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ChosenEvent : Event {
    sealed interface Error : ChosenEvent {
        data object Execution : Error
    }
    sealed interface From : ChosenEvent {
        data object Chosen : From
    }
}
// --- State Machine (W3C SCXML) ---

class ChosenStateMachine(
) : StateMachineEngine<ChosenState, ChosenEvent>() {

    override val initialState: ChosenState = ChosenState.Speak

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ChosenState? = when (stateId) {
        "done" -> ChosenState.Done
        "speak" -> ChosenState.Speak
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ChosenState): String = when (state) {
        is ChosenState.Done -> "done"
        is ChosenState.Speak -> "speak"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ChosenState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: ChosenState): Int = when (state) {
        is ChosenState.Done -> 1
        is ChosenState.Speak -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ChosenEvent? = when (name) {
        "error.execution" -> ChosenEvent.Error.Execution
        "from.chosen" -> ChosenEvent.From.Chosen
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ChosenEvent): String? = when (event) {
        is ChosenEvent.Error.Execution -> "error.execution"
        is ChosenEvent.From.Chosen -> "from.chosen"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: ChosenState,
        event: ChosenEvent
    ): TransitionResult<ChosenState> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: ChosenState
    ): TransitionResult<ChosenState> = when (state) {
        is ChosenState.Speak -> processNullSpeak()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSpeak(
    ): TransitionResult<ChosenState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(ChosenState.Done, ChosenState.Speak, 0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun onEntry(state: ChosenState, pathChild: ChosenState?) {
        when (state) {
            is ChosenState.Done -> {
                // SCE-MAP: chosen.scxml:13 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ChosenState.Speak -> {
                // SCE-MAP: chosen.scxml:7 :: speak :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("speak")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("from.chosen", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun onExit(state: ChosenState) {
        when (state) {
            is ChosenState.Done -> {
                // SCE-MAP: chosen.scxml:13 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is ChosenState.Speak -> {
                // SCE-MAP: chosen.scxml:7 :: speak :: _state_body
                activeStateIds.remove("speak")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: ChosenState,
        event: ChosenEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
