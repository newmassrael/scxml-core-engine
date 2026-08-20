// SCE-GENERATED — DO NOT EDIT
// source-hash: 7c010da1526dce3962148a99023f795b5efd3dc066529da8bc2dc12378934900
// template-hash: 90ac0b7250dd34a7e14136bc481cc93d6f1302dcf207c461738cfaee4b475c98
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_external_dequeue.scxml:52 :: _machine

package com.sce.integration.invoke_precedes_external_dequeue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesExternalDequeueState : State {
    data object Fail : InvokePrecedesExternalDequeueState
    data object Pass : InvokePrecedesExternalDequeueState
    data object Phase : InvokePrecedesExternalDequeueState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesExternalDequeueEvent : Event {
    sealed interface Cancel : InvokePrecedesExternalDequeueEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : InvokePrecedesExternalDequeueEvent {
        data object Invoke : Done
    }
    sealed interface Error : InvokePrecedesExternalDequeueEvent {
        data object Execution : Error
    }
    data object Kick : InvokePrecedesExternalDequeueEvent
    data object Probe : InvokePrecedesExternalDequeueEvent
    data object Ready : InvokePrecedesExternalDequeueEvent
    data object SawKick : InvokePrecedesExternalDequeueEvent
    data object SawNoKick : InvokePrecedesExternalDequeueEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesExternalDequeueStateMachine(
) : StateMachineEngine<InvokePrecedesExternalDequeueState, InvokePrecedesExternalDequeueEvent>() {

    override val initialState: InvokePrecedesExternalDequeueState = InvokePrecedesExternalDequeueState.Phase

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesExternalDequeueState? = when (stateId) {
        "fail" -> InvokePrecedesExternalDequeueState.Fail
        "pass" -> InvokePrecedesExternalDequeueState.Pass
        "phase" -> InvokePrecedesExternalDequeueState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesExternalDequeueState): String = when (state) {
        is InvokePrecedesExternalDequeueState.Fail -> "fail"
        is InvokePrecedesExternalDequeueState.Pass -> "pass"
        is InvokePrecedesExternalDequeueState.Phase -> "phase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokePrecedesExternalDequeueState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokePrecedesExternalDequeueState): Int = when (state) {
        is InvokePrecedesExternalDequeueState.Fail -> 2
        is InvokePrecedesExternalDequeueState.Pass -> 1
        is InvokePrecedesExternalDequeueState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesExternalDequeueEvent? = when (name) {
        "cancel.invoke" -> InvokePrecedesExternalDequeueEvent.Cancel.Invoke
        "done.invoke" -> InvokePrecedesExternalDequeueEvent.Done.Invoke
        "error.execution" -> InvokePrecedesExternalDequeueEvent.Error.Execution
        "kick" -> InvokePrecedesExternalDequeueEvent.Kick
        "probe" -> InvokePrecedesExternalDequeueEvent.Probe
        "ready" -> InvokePrecedesExternalDequeueEvent.Ready
        "sawKick" -> InvokePrecedesExternalDequeueEvent.SawKick
        "sawNoKick" -> InvokePrecedesExternalDequeueEvent.SawNoKick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesExternalDequeueEvent): String? = when (event) {
        is InvokePrecedesExternalDequeueEvent.Cancel.Invoke -> "cancel.invoke"
        is InvokePrecedesExternalDequeueEvent.Done.Invoke -> "done.invoke"
        is InvokePrecedesExternalDequeueEvent.Error.Execution -> "error.execution"
        is InvokePrecedesExternalDequeueEvent.Kick -> "kick"
        is InvokePrecedesExternalDequeueEvent.Probe -> "probe"
        is InvokePrecedesExternalDequeueEvent.Ready -> "ready"
        is InvokePrecedesExternalDequeueEvent.SawKick -> "sawKick"
        is InvokePrecedesExternalDequeueEvent.SawNoKick -> "sawNoKick"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokePrecedesExternalDequeueState,
        event: InvokePrecedesExternalDequeueEvent
    ): TransitionResult<InvokePrecedesExternalDequeueState> = when (state) {
        is InvokePrecedesExternalDequeueState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: InvokePrecedesExternalDequeueEvent
    ): TransitionResult<InvokePrecedesExternalDequeueState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InvokePrecedesExternalDequeueEvent.Kick -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InvokePrecedesExternalDequeueEvent.Ready -> TransitionResult.Internal
        event is InvokePrecedesExternalDequeueEvent.SawKick -> TransitionResult.External(InvokePrecedesExternalDequeueState.Pass, InvokePrecedesExternalDequeueState.Phase)

        event is InvokePrecedesExternalDequeueEvent.SawNoKick -> TransitionResult.External(InvokePrecedesExternalDequeueState.Fail, InvokePrecedesExternalDequeueState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:52 :: _machine
    override fun onEntry(state: InvokePrecedesExternalDequeueState, pathChild: InvokePrecedesExternalDequeueState?) {
        when (state) {
            is InvokePrecedesExternalDequeueState.Fail -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:92 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesExternalDequeueState.Pass -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:91 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesExternalDequeueState.Phase -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:55 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return


            send(InvokePrecedesExternalDequeueEvent.Kick, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_watch"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokePrecedesExternalDequeueSceSynthInvokeInvWatchStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_watch", childSM, true, InvokePrecedesExternalDequeueEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:52 :: _machine
    override fun onExit(state: InvokePrecedesExternalDequeueState) {
        when (state) {
            is InvokePrecedesExternalDequeueState.Fail -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:92 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is InvokePrecedesExternalDequeueState.Pass -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:91 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokePrecedesExternalDequeueState.Phase -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:55 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:52 :: _machine
    override fun executeTransitionActions(
        source: InvokePrecedesExternalDequeueState,
        event: InvokePrecedesExternalDequeueEvent?
    ) {
        when (source) {
        is InvokePrecedesExternalDequeueState.Phase -> when {
            event is InvokePrecedesExternalDequeueEvent.Ready -> {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:85 :: phase :: _transition_1


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_watch", "probe")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
