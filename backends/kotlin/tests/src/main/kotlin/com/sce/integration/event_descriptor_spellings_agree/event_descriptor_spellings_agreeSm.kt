// SCE-GENERATED — DO NOT EDIT
// source-hash: 50d20d99cbb48f12a0880885cd1f0df5d9e67ae77d666c0c74ded8f10b4a9982
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine

package com.sce.integration.event_descriptor_spellings_agree

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventDescriptorSpellingsAgreeState : State {
    data object Bounded : EventDescriptorSpellingsAgreeState
    data object Dotted : EventDescriptorSpellingsAgreeState
    data object FailBounded : EventDescriptorSpellingsAgreeState
    data object FailDotted : EventDescriptorSpellingsAgreeState
    data object FailSuffixed : EventDescriptorSpellingsAgreeState
    data object FailUniversal : EventDescriptorSpellingsAgreeState
    data object Pass : EventDescriptorSpellingsAgreeState
    data object Suffixed : EventDescriptorSpellingsAgreeState
    data object Universal : EventDescriptorSpellingsAgreeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventDescriptorSpellingsAgreeEvent : Event {
    sealed interface Any : EventDescriptorSpellingsAgreeEvent {
        sealed interface Token : Any {
            data object Sequence : Token
        }
    }
    data object Dot : EventDescriptorSpellingsAgreeEvent
    data object Wild : EventDescriptorSpellingsAgreeEvent
    data object Wilder : EventDescriptorSpellingsAgreeEvent
}
// --- State Machine (W3C SCXML) ---

class EventDescriptorSpellingsAgreeStateMachine(
) : StateMachineEngine<EventDescriptorSpellingsAgreeState, EventDescriptorSpellingsAgreeEvent>() {

    override val initialState: EventDescriptorSpellingsAgreeState = EventDescriptorSpellingsAgreeState.Suffixed

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EventDescriptorSpellingsAgreeState? = when (stateId) {
        "bounded" -> EventDescriptorSpellingsAgreeState.Bounded
        "dotted" -> EventDescriptorSpellingsAgreeState.Dotted
        "failBounded" -> EventDescriptorSpellingsAgreeState.FailBounded
        "failDotted" -> EventDescriptorSpellingsAgreeState.FailDotted
        "failSuffixed" -> EventDescriptorSpellingsAgreeState.FailSuffixed
        "failUniversal" -> EventDescriptorSpellingsAgreeState.FailUniversal
        "pass" -> EventDescriptorSpellingsAgreeState.Pass
        "suffixed" -> EventDescriptorSpellingsAgreeState.Suffixed
        "universal" -> EventDescriptorSpellingsAgreeState.Universal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventDescriptorSpellingsAgreeState): String = when (state) {
        is EventDescriptorSpellingsAgreeState.Bounded -> "bounded"
        is EventDescriptorSpellingsAgreeState.Dotted -> "dotted"
        is EventDescriptorSpellingsAgreeState.FailBounded -> "failBounded"
        is EventDescriptorSpellingsAgreeState.FailDotted -> "failDotted"
        is EventDescriptorSpellingsAgreeState.FailSuffixed -> "failSuffixed"
        is EventDescriptorSpellingsAgreeState.FailUniversal -> "failUniversal"
        is EventDescriptorSpellingsAgreeState.Pass -> "pass"
        is EventDescriptorSpellingsAgreeState.Suffixed -> "suffixed"
        is EventDescriptorSpellingsAgreeState.Universal -> "universal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventDescriptorSpellingsAgreeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventDescriptorSpellingsAgreeState): Int = when (state) {
        is EventDescriptorSpellingsAgreeState.Bounded -> 2
        is EventDescriptorSpellingsAgreeState.Dotted -> 1
        is EventDescriptorSpellingsAgreeState.FailBounded -> 7
        is EventDescriptorSpellingsAgreeState.FailDotted -> 6
        is EventDescriptorSpellingsAgreeState.FailSuffixed -> 5
        is EventDescriptorSpellingsAgreeState.FailUniversal -> 8
        is EventDescriptorSpellingsAgreeState.Pass -> 4
        is EventDescriptorSpellingsAgreeState.Suffixed -> 0
        is EventDescriptorSpellingsAgreeState.Universal -> 3
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: EventDescriptorSpellingsAgreeState,
        event: EventDescriptorSpellingsAgreeEvent
    ): TransitionResult<EventDescriptorSpellingsAgreeState> = when (state) {
        is EventDescriptorSpellingsAgreeState.Bounded -> processBounded(event)
        is EventDescriptorSpellingsAgreeState.Dotted -> processDotted(event)
        is EventDescriptorSpellingsAgreeState.Suffixed -> processSuffixed(event)
        is EventDescriptorSpellingsAgreeState.Universal -> processUniversal(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processBounded(
        event: EventDescriptorSpellingsAgreeEvent
    ): TransitionResult<EventDescriptorSpellingsAgreeState> = when {
        event is EventDescriptorSpellingsAgreeEvent.Wild -> TransitionResult.External(EventDescriptorSpellingsAgreeState.FailBounded, EventDescriptorSpellingsAgreeState.Bounded, 0)

        event is EventDescriptorSpellingsAgreeEvent.Wilder -> TransitionResult.External(EventDescriptorSpellingsAgreeState.Universal, EventDescriptorSpellingsAgreeState.Bounded, 1)

        else -> TransitionResult.Ignored
    }

    private fun processDotted(
        event: EventDescriptorSpellingsAgreeEvent
    ): TransitionResult<EventDescriptorSpellingsAgreeState> = when {
        event is EventDescriptorSpellingsAgreeEvent.Dot -> TransitionResult.External(EventDescriptorSpellingsAgreeState.Bounded, EventDescriptorSpellingsAgreeState.Dotted, 2)

        event is EventDescriptorSpellingsAgreeEvent.Dot -> TransitionResult.External(EventDescriptorSpellingsAgreeState.FailDotted, EventDescriptorSpellingsAgreeState.Dotted, 3)

        else -> TransitionResult.Ignored
    }

    private fun processSuffixed(
        event: EventDescriptorSpellingsAgreeEvent
    ): TransitionResult<EventDescriptorSpellingsAgreeState> = when {
        event is EventDescriptorSpellingsAgreeEvent.Wild -> TransitionResult.External(EventDescriptorSpellingsAgreeState.Dotted, EventDescriptorSpellingsAgreeState.Suffixed, 4)

        event is EventDescriptorSpellingsAgreeEvent.Wild -> TransitionResult.External(EventDescriptorSpellingsAgreeState.FailSuffixed, EventDescriptorSpellingsAgreeState.Suffixed, 5)

        else -> TransitionResult.Ignored
    }

    private fun processUniversal(
        event: EventDescriptorSpellingsAgreeEvent
    ): TransitionResult<EventDescriptorSpellingsAgreeState> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(EventDescriptorSpellingsAgreeState.Pass, EventDescriptorSpellingsAgreeState.Universal, 6)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine
    override fun onEntry(state: EventDescriptorSpellingsAgreeState, pathChild: EventDescriptorSpellingsAgreeState?) {
        when (state) {
            is EventDescriptorSpellingsAgreeState.Bounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:76 :: bounded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("bounded")) return

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Wilder)
            }
            is EventDescriptorSpellingsAgreeState.Dotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:68 :: dotted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("dotted")) return

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Dot)
            }
            is EventDescriptorSpellingsAgreeState.FailBounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:95 :: failBounded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failBounded")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailDotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:94 :: failDotted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failDotted")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailSuffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:93 :: failSuffixed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failSuffixed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailUniversal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:96 :: failUniversal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failUniversal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.Pass -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:92 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.Suffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:60 :: suffixed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("suffixed")) return

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Wild)
            }
            is EventDescriptorSpellingsAgreeState.Universal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:84 :: universal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("universal")) return

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Any.Token.Sequence)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine
    override fun onExit(state: EventDescriptorSpellingsAgreeState) {
        when (state) {
            is EventDescriptorSpellingsAgreeState.Bounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:76 :: bounded :: _state_body
                activeStateIds.remove("bounded")
            }
            is EventDescriptorSpellingsAgreeState.Dotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:68 :: dotted :: _state_body
                activeStateIds.remove("dotted")
            }
            is EventDescriptorSpellingsAgreeState.FailBounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:95 :: failBounded :: _state_body
                activeStateIds.remove("failBounded")
            }
            is EventDescriptorSpellingsAgreeState.FailDotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:94 :: failDotted :: _state_body
                activeStateIds.remove("failDotted")
            }
            is EventDescriptorSpellingsAgreeState.FailSuffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:93 :: failSuffixed :: _state_body
                activeStateIds.remove("failSuffixed")
            }
            is EventDescriptorSpellingsAgreeState.FailUniversal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:96 :: failUniversal :: _state_body
                activeStateIds.remove("failUniversal")
            }
            is EventDescriptorSpellingsAgreeState.Pass -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:92 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is EventDescriptorSpellingsAgreeState.Suffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:60 :: suffixed :: _state_body
                activeStateIds.remove("suffixed")
            }
            is EventDescriptorSpellingsAgreeState.Universal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:84 :: universal :: _state_body
                activeStateIds.remove("universal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine
    override fun executeTransitionActions(
        source: EventDescriptorSpellingsAgreeState,
        event: EventDescriptorSpellingsAgreeEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
