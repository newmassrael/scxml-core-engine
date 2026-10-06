// SCE-GENERATED — DO NOT EDIT
// source-hash: 64fb223807f4c558ee24bbcd3b67b8ca6498504cd273de737c75cdc2b6d599ee

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke__sce_synth_invoke__worker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:3 :: _machine

package com.sce.integration.static_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeSceSynthInvokeWorkerState : State {
    data object First : StaticInvokeSceSynthInvokeWorkerState
    data object Leaf : StaticInvokeSceSynthInvokeWorkerState
    data object Second : StaticInvokeSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeSceSynthInvokeWorkerEvent : Event {
    data object A : StaticInvokeSceSynthInvokeWorkerEvent
    data object B : StaticInvokeSceSynthInvokeWorkerEvent
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeSceSynthInvokeWorkerStateMachine(
) : StateMachineEngine<StaticInvokeSceSynthInvokeWorkerState, StaticInvokeSceSynthInvokeWorkerEvent>() {

    override val initialState: StaticInvokeSceSynthInvokeWorkerState = StaticInvokeSceSynthInvokeWorkerState.First

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticInvokeSceSynthInvokeWorkerState): Boolean = when (state) {
        is StaticInvokeSceSynthInvokeWorkerState.Leaf -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(StaticInvokeSceSynthInvokeWorkerState.First))

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<StaticInvokeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeSceSynthInvokeWorkerState.First,
            listOf(StateTarget(StaticInvokeSceSynthInvokeWorkerState.Second)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<StaticInvokeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeSceSynthInvokeWorkerState.Second,
            listOf(StateTarget(StaticInvokeSceSynthInvokeWorkerState.Leaf)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeSceSynthInvokeWorkerState? = when (stateId) {
        "first" -> StaticInvokeSceSynthInvokeWorkerState.First
        "leaf" -> StaticInvokeSceSynthInvokeWorkerState.Leaf
        "second" -> StaticInvokeSceSynthInvokeWorkerState.Second
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeSceSynthInvokeWorkerState): String = when (state) {
        is StaticInvokeSceSynthInvokeWorkerState.First -> "first"
        is StaticInvokeSceSynthInvokeWorkerState.Leaf -> "leaf"
        is StaticInvokeSceSynthInvokeWorkerState.Second -> "second"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeSceSynthInvokeWorkerState): Int = when (state) {
        is StaticInvokeSceSynthInvokeWorkerState.First -> 0
        is StaticInvokeSceSynthInvokeWorkerState.Leaf -> 2
        is StaticInvokeSceSynthInvokeWorkerState.Second -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeSceSynthInvokeWorkerEvent? = when (name) {
        "a" -> StaticInvokeSceSynthInvokeWorkerEvent.A
        "b" -> StaticInvokeSceSynthInvokeWorkerEvent.B
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeSceSynthInvokeWorkerEvent): String? = when (event) {
        is StaticInvokeSceSynthInvokeWorkerEvent.A -> "a"
        is StaticInvokeSceSynthInvokeWorkerEvent.B -> "b"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeSceSynthInvokeWorkerState,
        event: StaticInvokeSceSynthInvokeWorkerEvent?
    ): EnabledTransition<StaticInvokeSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is StaticInvokeSceSynthInvokeWorkerState.First -> when {
            event is StaticInvokeSceSynthInvokeWorkerEvent.A -> transitionFirstAt0
            else -> null
        }
        is StaticInvokeSceSynthInvokeWorkerState.Second -> when {
            event is StaticInvokeSceSynthInvokeWorkerEvent.B -> transitionSecondAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onEntry(state: StaticInvokeSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:4 :: first :: _state_body
            }
            is StaticInvokeSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:10 :: leaf :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticInvokeSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:7 :: second :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onExit(state: StaticInvokeSceSynthInvokeWorkerState) {
        when (state) {
            is StaticInvokeSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:4 :: first :: _state_body
            }
            is StaticInvokeSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:10 :: leaf :: _state_body
            }
            is StaticInvokeSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:7 :: second :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticInvokeSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
