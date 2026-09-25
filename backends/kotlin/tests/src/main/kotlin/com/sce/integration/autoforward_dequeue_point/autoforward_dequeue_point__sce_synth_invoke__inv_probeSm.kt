// SCE-GENERATED — DO NOT EDIT
// source-hash: ce55909c83cc4666c5ceb48ddcf2f5ce650a9da03007b3cc081cde9b3ac0761e

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine

package com.sce.integration.autoforward_dequeue_point

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDequeuePointSceSynthInvokeInvProbeState : State {
    data object Awaiting : AutoforwardDequeuePointSceSynthInvokeInvProbeState
    data object Early : AutoforwardDequeuePointSceSynthInvokeInvProbeState
    data object Marked : AutoforwardDequeuePointSceSynthInvokeInvProbeState
    data object Ordered : AutoforwardDequeuePointSceSynthInvokeInvProbeState
    data object Probe : AutoforwardDequeuePointSceSynthInvokeInvProbeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDequeuePointSceSynthInvokeInvProbeEvent : Event {
    sealed interface Error : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent {
        data object Execution : Error
    }
    data object First : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    data object Mark : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    data object Ready : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    data object SawMarkFirst : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    data object SawSecondEarly : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    data object Second : AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardDequeuePointSceSynthInvokeInvProbeStateMachine(
) : StateMachineEngine<AutoforwardDequeuePointSceSynthInvokeInvProbeState, AutoforwardDequeuePointSceSynthInvokeInvProbeEvent>() {

    override val initialState: AutoforwardDequeuePointSceSynthInvokeInvProbeState = AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe

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
    override fun isFinalState(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState): Boolean = when (state) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early, is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>> =
            listOf(StateTarget(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe))

        // W3C SCXML 3.13: awaiting's transition 0, as the microstep reads it.
        val transitionAwaitingAt0 = EnabledTransition<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>(
            AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting,
            listOf(StateTarget(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaiting's transition 1, as the microstep reads it.
        val transitionAwaitingAt1 = EnabledTransition<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>(
            AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting,
            listOf(StateTarget(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: marked's transition 0, as the microstep reads it.
        val transitionMarkedAt0 = EnabledTransition<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>(
            AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked,
            listOf(StateTarget(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: probe's transition 0, as the microstep reads it.
        val transitionProbeAt0 = EnabledTransition<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>(
            AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe,
            listOf(StateTarget(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDequeuePointSceSynthInvokeInvProbeState? = when (stateId) {
        "awaiting" -> AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting
        "early" -> AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early
        "marked" -> AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked
        "ordered" -> AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered
        "probe" -> AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState): String = when (state) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> "awaiting"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> "early"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> "marked"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> "ordered"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> "probe"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState): Int = when (state) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> 1
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> 3
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> 2
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> 4
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDequeuePointSceSynthInvokeInvProbeEvent? = when (name) {
        "error.execution" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Error.Execution
        "first" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.First
        "mark" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Mark
        "ready" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Ready
        "sawMarkFirst" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.SawMarkFirst
        "sawSecondEarly" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.SawSecondEarly
        "second" -> AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent): String? = when (event) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Error.Execution -> "error.execution"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.First -> "first"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Mark -> "mark"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Ready -> "ready"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.SawMarkFirst -> "sawMarkFirst"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.SawSecondEarly -> "sawSecondEarly"
        is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> "second"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardDequeuePointSceSynthInvokeInvProbeState,
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent?
    ): EnabledTransition<AutoforwardDequeuePointSceSynthInvokeInvProbeState, HistoryId>? = when (state) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> when {
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Mark -> transitionAwaitingAt0
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> transitionAwaitingAt1
            else -> null
        }
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> when {
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> transitionMarkedAt0
            else -> null
        }
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> when {
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.First -> transitionProbeAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:11 :: awaiting :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:22 :: early :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:17 :: marked :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:23 :: ordered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:5 :: probe :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onExit(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState) {
        when (state) {
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:11 :: awaiting :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:22 :: early :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:17 :: marked :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:23 :: ordered :: _state_body
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:5 :: probe :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun executeTransitionContent(source: AutoforwardDequeuePointSceSynthInvokeInvProbeState, transitionIndex: Int) {
        when (source) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> when (transitionIndex) {
            1 -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:13 :: awaiting :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawSecondEarly", "")
            }
            else -> {}
        }
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:18 :: marked :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawMarkFirst", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
