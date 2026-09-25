// SCE-GENERATED — DO NOT EDIT
// source-hash: 50d20d99cbb48f12a0880885cd1f0df5d9e67ae77d666c0c74ded8f10b4a9982

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: EventDescriptorSpellingsAgreeState): Boolean = when (state) {
        is EventDescriptorSpellingsAgreeState.FailBounded, is EventDescriptorSpellingsAgreeState.FailDotted, is EventDescriptorSpellingsAgreeState.FailSuffixed, is EventDescriptorSpellingsAgreeState.FailUniversal, is EventDescriptorSpellingsAgreeState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EventDescriptorSpellingsAgreeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<EventDescriptorSpellingsAgreeState, HistoryId>> =
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.Suffixed))

        // W3C SCXML 3.13: bounded's transition 0, as the microstep reads it.
        val transitionBoundedAt0 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Bounded,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.FailBounded)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 1, as the microstep reads it.
        val transitionBoundedAt1 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Bounded,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.Universal)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: dotted's transition 0, as the microstep reads it.
        val transitionDottedAt0 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Dotted,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.Bounded)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: dotted's transition 1, as the microstep reads it.
        val transitionDottedAt1 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Dotted,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.FailDotted)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: suffixed's transition 0, as the microstep reads it.
        val transitionSuffixedAt0 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Suffixed,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.Dotted)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: suffixed's transition 1, as the microstep reads it.
        val transitionSuffixedAt1 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Suffixed,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.FailSuffixed)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: universal's transition 0, as the microstep reads it.
        val transitionUniversalAt0 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Universal,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: universal's transition 1, as the microstep reads it.
        val transitionUniversalAt1 = EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>(
            EventDescriptorSpellingsAgreeState.Universal,
            listOf(StateTarget(EventDescriptorSpellingsAgreeState.FailUniversal)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: EventDescriptorSpellingsAgreeState,
        event: EventDescriptorSpellingsAgreeEvent?
    ): EnabledTransition<EventDescriptorSpellingsAgreeState, HistoryId>? = when (state) {
        is EventDescriptorSpellingsAgreeState.Bounded -> when {
            event is EventDescriptorSpellingsAgreeEvent.Wild -> transitionBoundedAt0
            event is EventDescriptorSpellingsAgreeEvent.Wilder -> transitionBoundedAt1
            else -> null
        }
        is EventDescriptorSpellingsAgreeState.Dotted -> when {
            event is EventDescriptorSpellingsAgreeEvent.Dot -> transitionDottedAt0
            event is EventDescriptorSpellingsAgreeEvent.Dot -> transitionDottedAt1
            else -> null
        }
        is EventDescriptorSpellingsAgreeState.Suffixed -> when {
            event is EventDescriptorSpellingsAgreeEvent.Wild -> transitionSuffixedAt0
            event is EventDescriptorSpellingsAgreeEvent.Wild -> transitionSuffixedAt1
            else -> null
        }
        is EventDescriptorSpellingsAgreeState.Universal -> when {
            event != null -> transitionUniversalAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine
    override fun onEntry(state: EventDescriptorSpellingsAgreeState, isDefaultEntry: Boolean) {
        when (state) {
            is EventDescriptorSpellingsAgreeState.Bounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:76 :: bounded :: _state_body

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Wilder)
            }
            is EventDescriptorSpellingsAgreeState.Dotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:68 :: dotted :: _state_body

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Dot)
            }
            is EventDescriptorSpellingsAgreeState.FailBounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:95 :: failBounded :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailDotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:94 :: failDotted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailSuffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:93 :: failSuffixed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.FailUniversal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:96 :: failUniversal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.Pass -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:92 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDescriptorSpellingsAgreeState.Suffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:60 :: suffixed :: _state_body

            raiseInternal(EventDescriptorSpellingsAgreeEvent.Wild)
            }
            is EventDescriptorSpellingsAgreeState.Universal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:84 :: universal :: _state_body

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
            }
            is EventDescriptorSpellingsAgreeState.Dotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:68 :: dotted :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.FailBounded -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:95 :: failBounded :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.FailDotted -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:94 :: failDotted :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.FailSuffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:93 :: failSuffixed :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.FailUniversal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:96 :: failUniversal :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.Pass -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:92 :: pass :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.Suffixed -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:60 :: suffixed :: _state_body
            }
            is EventDescriptorSpellingsAgreeState.Universal -> {
                // SCE-MAP: event_descriptor_spellings_agree.scxml:84 :: universal :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: event_descriptor_spellings_agree.scxml:56 :: _machine
    override fun executeTransitionContent(source: EventDescriptorSpellingsAgreeState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
