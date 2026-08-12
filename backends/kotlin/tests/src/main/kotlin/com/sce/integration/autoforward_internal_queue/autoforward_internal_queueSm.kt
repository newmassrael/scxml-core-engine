// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c78d9a40e778435f5ba721a7a12bf6721453dde3c80246e5018de3fc670010
// template-hash: 74ba562b33766da248288b5dadec1e79a0ebb46a66e38786f6a7a4b2ccd653e3
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine

package com.sce.integration.autoforward_internal_queue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardInternalQueueState : State {
    data object Fail : AutoforwardInternalQueueState
    data object Pass : AutoforwardInternalQueueState
    data object Phase : AutoforwardInternalQueueState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardInternalQueueEvent : Event {
    data object Boom : AutoforwardInternalQueueEvent
    sealed interface Cancel : AutoforwardInternalQueueEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : AutoforwardInternalQueueEvent {
        data object Invoke : Done
    }
    sealed interface Error : AutoforwardInternalQueueEvent {
        data object Execution : Error
    }
    data object Probe : AutoforwardInternalQueueEvent
    data object Ready : AutoforwardInternalQueueEvent
    data object SawInternal : AutoforwardInternalQueueEvent
    data object SawProbeOnly : AutoforwardInternalQueueEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardInternalQueueStateMachine(
) : StateMachineEngine<AutoforwardInternalQueueState, AutoforwardInternalQueueEvent>() {

    override val initialState: AutoforwardInternalQueueState = AutoforwardInternalQueueState.Phase



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardInternalQueueState? = when (stateId) {
        "fail" -> AutoforwardInternalQueueState.Fail
        "pass" -> AutoforwardInternalQueueState.Pass
        "phase" -> AutoforwardInternalQueueState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardInternalQueueState): String = when (state) {
        is AutoforwardInternalQueueState.Fail -> "fail"
        is AutoforwardInternalQueueState.Pass -> "pass"
        is AutoforwardInternalQueueState.Phase -> "phase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardInternalQueueState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardInternalQueueState): Int = when (state) {
        is AutoforwardInternalQueueState.Fail -> 2
        is AutoforwardInternalQueueState.Pass -> 1
        is AutoforwardInternalQueueState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardInternalQueueEvent? = when (name) {
        "boom" -> AutoforwardInternalQueueEvent.Boom
        "cancel.invoke" -> AutoforwardInternalQueueEvent.Cancel.Invoke
        "done.invoke" -> AutoforwardInternalQueueEvent.Done.Invoke
        "error.execution" -> AutoforwardInternalQueueEvent.Error.Execution
        "probe" -> AutoforwardInternalQueueEvent.Probe
        "ready" -> AutoforwardInternalQueueEvent.Ready
        "sawInternal" -> AutoforwardInternalQueueEvent.SawInternal
        "sawProbeOnly" -> AutoforwardInternalQueueEvent.SawProbeOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardInternalQueueEvent): String? = when (event) {
        is AutoforwardInternalQueueEvent.Boom -> "boom"
        is AutoforwardInternalQueueEvent.Cancel.Invoke -> "cancel.invoke"
        is AutoforwardInternalQueueEvent.Done.Invoke -> "done.invoke"
        is AutoforwardInternalQueueEvent.Error.Execution -> "error.execution"
        is AutoforwardInternalQueueEvent.Probe -> "probe"
        is AutoforwardInternalQueueEvent.Ready -> "ready"
        is AutoforwardInternalQueueEvent.SawInternal -> "sawInternal"
        is AutoforwardInternalQueueEvent.SawProbeOnly -> "sawProbeOnly"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardInternalQueueState,
        event: AutoforwardInternalQueueEvent
    ): TransitionResult<AutoforwardInternalQueueState> = when (state) {
        is AutoforwardInternalQueueState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: AutoforwardInternalQueueEvent
    ): TransitionResult<AutoforwardInternalQueueState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardInternalQueueEvent.Ready -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardInternalQueueEvent.Error.Execution -> TransitionResult.Internal
        event is AutoforwardInternalQueueEvent.SawInternal -> TransitionResult.External(AutoforwardInternalQueueState.Fail, AutoforwardInternalQueueState.Phase)

        event is AutoforwardInternalQueueEvent.SawProbeOnly -> TransitionResult.External(AutoforwardInternalQueueState.Pass, AutoforwardInternalQueueState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine
    override fun onEntry(state: AutoforwardInternalQueueState) {
        when (state) {
            is AutoforwardInternalQueueState.Fail -> {
                // SCE-MAP: autoforward_internal_queue.scxml:85 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueState.Pass -> {
                // SCE-MAP: autoforward_internal_queue.scxml:84 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueState.Phase -> {
                // SCE-MAP: autoforward_internal_queue.scxml:54 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_watch"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = AutoforwardInternalQueueSceSynthInvokeInvWatchStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_watch", childSM, true, AutoforwardInternalQueueEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine
    override fun onExit(state: AutoforwardInternalQueueState) {
        when (state) {
            is AutoforwardInternalQueueState.Fail -> {
                // SCE-MAP: autoforward_internal_queue.scxml:85 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is AutoforwardInternalQueueState.Pass -> {
                // SCE-MAP: autoforward_internal_queue.scxml:84 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is AutoforwardInternalQueueState.Phase -> {
                // SCE-MAP: autoforward_internal_queue.scxml:54 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine
    override fun executeTransitionActions(
        source: AutoforwardInternalQueueState,
        event: AutoforwardInternalQueueEvent?
    ) {
        when (source) {
        is AutoforwardInternalQueueState.Phase -> when {
            event is AutoforwardInternalQueueEvent.Ready -> {
                // SCE-MAP: autoforward_internal_queue.scxml:75 :: phase :: _transition_0


            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raiseInternal(AutoforwardInternalQueueEvent.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
            event is AutoforwardInternalQueueEvent.Error.Execution -> {
                // SCE-MAP: autoforward_internal_queue.scxml:78 :: phase :: _transition_1


            send(AutoforwardInternalQueueEvent.Probe, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
