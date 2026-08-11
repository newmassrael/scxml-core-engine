// SCE-GENERATED — DO NOT EDIT
// source-hash: ce55909c83cc4666c5ceb48ddcf2f5ce650a9da03007b3cc081cde9b3ac0761e
// template-hash: 56bec87d0124f368b72ecb45f170dc38a324027a2fa3663195c8aeaa13f5d24d
// generated-at: 0

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardDequeuePointSceSynthInvokeInvProbeState,
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    ): TransitionResult<AutoforwardDequeuePointSceSynthInvokeInvProbeState> = when (state) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> processAwaiting(event)
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> processMarked(event)
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> processProbe(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processAwaiting(
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    ): TransitionResult<AutoforwardDequeuePointSceSynthInvokeInvProbeState> = when {
        event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Mark -> TransitionResult.External(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked, AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting)

        event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> TransitionResult.External(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early, AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting)

        else -> TransitionResult.Ignored
    }

    private fun processMarked(
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    ): TransitionResult<AutoforwardDequeuePointSceSynthInvokeInvProbeState> = when {
        event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> TransitionResult.External(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered, AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked)

        else -> TransitionResult.Ignored
    }

    private fun processProbe(
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent
    ): TransitionResult<AutoforwardDequeuePointSceSynthInvokeInvProbeState> = when {
        event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.First -> TransitionResult.External(AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting, AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardDequeuePointSceSynthInvokeInvProbeState) {
        when (state) {
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:11 :: awaiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("awaiting")) return
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:22 :: early :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("early")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:17 :: marked :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("marked")) return
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:23 :: ordered :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ordered")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:5 :: probe :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("probe")) return


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
                activeStateIds.remove("awaiting")
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Early -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:22 :: early :: _state_body
                activeStateIds.remove("early")
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:17 :: marked :: _state_body
                activeStateIds.remove("marked")
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Ordered -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:23 :: ordered :: _state_body
                activeStateIds.remove("ordered")
            }
            is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Probe -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:5 :: probe :: _state_body
                activeStateIds.remove("probe")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: AutoforwardDequeuePointSceSynthInvokeInvProbeState,
        event: AutoforwardDequeuePointSceSynthInvokeInvProbeEvent?
    ) {
        when (source) {
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Awaiting -> when {
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> {
                // SCE-MAP: autoforward_dequeue_point__sce_synth_invoke__inv_probe.scxml:13 :: awaiting :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawSecondEarly", "")
            }
            else -> {}
        }
        is AutoforwardDequeuePointSceSynthInvokeInvProbeState.Marked -> when {
            event is AutoforwardDequeuePointSceSynthInvokeInvProbeEvent.Second -> {
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
