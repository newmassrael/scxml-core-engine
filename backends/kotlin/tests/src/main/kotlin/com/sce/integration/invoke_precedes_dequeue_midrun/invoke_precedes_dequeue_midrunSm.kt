// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine

package com.sce.integration.invoke_precedes_dequeue_midrun

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesDequeueMidrunState : State {
    data object Arm : InvokePrecedesDequeueMidrunState
    data object Fail : InvokePrecedesDequeueMidrunState
    data object Pass : InvokePrecedesDequeueMidrunState
    data object Phase : InvokePrecedesDequeueMidrunState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesDequeueMidrunEvent : Event {
    sealed interface Cancel : InvokePrecedesDequeueMidrunEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : InvokePrecedesDequeueMidrunEvent {
        data object Invoke : Done
    }
    sealed interface Error : InvokePrecedesDequeueMidrunEvent {
        data object Execution : Error
    }
    data object Go : InvokePrecedesDequeueMidrunEvent
    data object Kick : InvokePrecedesDequeueMidrunEvent
    data object Probe : InvokePrecedesDequeueMidrunEvent
    data object Ready : InvokePrecedesDequeueMidrunEvent
    data object SawKick : InvokePrecedesDequeueMidrunEvent
    data object SawNoKick : InvokePrecedesDequeueMidrunEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesDequeueMidrunStateMachine(
) : StateMachineEngine<InvokePrecedesDequeueMidrunState, InvokePrecedesDequeueMidrunEvent>() {

    override val initialState: InvokePrecedesDequeueMidrunState = InvokePrecedesDequeueMidrunState.Arm



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesDequeueMidrunState? = when (stateId) {
        "arm" -> InvokePrecedesDequeueMidrunState.Arm
        "fail" -> InvokePrecedesDequeueMidrunState.Fail
        "pass" -> InvokePrecedesDequeueMidrunState.Pass
        "phase" -> InvokePrecedesDequeueMidrunState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesDequeueMidrunState): String = when (state) {
        is InvokePrecedesDequeueMidrunState.Arm -> "arm"
        is InvokePrecedesDequeueMidrunState.Fail -> "fail"
        is InvokePrecedesDequeueMidrunState.Pass -> "pass"
        is InvokePrecedesDequeueMidrunState.Phase -> "phase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokePrecedesDequeueMidrunState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokePrecedesDequeueMidrunState): Int = when (state) {
        is InvokePrecedesDequeueMidrunState.Arm -> 0
        is InvokePrecedesDequeueMidrunState.Fail -> 3
        is InvokePrecedesDequeueMidrunState.Pass -> 2
        is InvokePrecedesDequeueMidrunState.Phase -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesDequeueMidrunEvent? = when (name) {
        "cancel.invoke" -> InvokePrecedesDequeueMidrunEvent.Cancel.Invoke
        "done.invoke" -> InvokePrecedesDequeueMidrunEvent.Done.Invoke
        "error.execution" -> InvokePrecedesDequeueMidrunEvent.Error.Execution
        "go" -> InvokePrecedesDequeueMidrunEvent.Go
        "kick" -> InvokePrecedesDequeueMidrunEvent.Kick
        "probe" -> InvokePrecedesDequeueMidrunEvent.Probe
        "ready" -> InvokePrecedesDequeueMidrunEvent.Ready
        "sawKick" -> InvokePrecedesDequeueMidrunEvent.SawKick
        "sawNoKick" -> InvokePrecedesDequeueMidrunEvent.SawNoKick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesDequeueMidrunEvent): String? = when (event) {
        is InvokePrecedesDequeueMidrunEvent.Cancel.Invoke -> "cancel.invoke"
        is InvokePrecedesDequeueMidrunEvent.Done.Invoke -> "done.invoke"
        is InvokePrecedesDequeueMidrunEvent.Error.Execution -> "error.execution"
        is InvokePrecedesDequeueMidrunEvent.Go -> "go"
        is InvokePrecedesDequeueMidrunEvent.Kick -> "kick"
        is InvokePrecedesDequeueMidrunEvent.Probe -> "probe"
        is InvokePrecedesDequeueMidrunEvent.Ready -> "ready"
        is InvokePrecedesDequeueMidrunEvent.SawKick -> "sawKick"
        is InvokePrecedesDequeueMidrunEvent.SawNoKick -> "sawNoKick"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokePrecedesDequeueMidrunState,
        event: InvokePrecedesDequeueMidrunEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunState> = when (state) {
        is InvokePrecedesDequeueMidrunState.Arm -> processArm(event)
        is InvokePrecedesDequeueMidrunState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processArm(
        event: InvokePrecedesDequeueMidrunEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunState> = when {
        event is InvokePrecedesDequeueMidrunEvent.Go -> TransitionResult.External(InvokePrecedesDequeueMidrunState.Phase, InvokePrecedesDequeueMidrunState.Arm)

        else -> TransitionResult.Ignored
    }

    private fun processPhase(
        event: InvokePrecedesDequeueMidrunEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InvokePrecedesDequeueMidrunEvent.Kick -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InvokePrecedesDequeueMidrunEvent.Ready -> TransitionResult.Internal
        event is InvokePrecedesDequeueMidrunEvent.SawKick -> TransitionResult.External(InvokePrecedesDequeueMidrunState.Pass, InvokePrecedesDequeueMidrunState.Phase)

        event is InvokePrecedesDequeueMidrunEvent.SawNoKick -> TransitionResult.External(InvokePrecedesDequeueMidrunState.Fail, InvokePrecedesDequeueMidrunState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine
    override fun onEntry(state: InvokePrecedesDequeueMidrunState, pathChild: InvokePrecedesDequeueMidrunState?) {
        when (state) {
            is InvokePrecedesDequeueMidrunState.Arm -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:45 :: arm :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("arm")) return


            send(InvokePrecedesDequeueMidrunEvent.Go, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:89 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:88 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:54 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return


            send(InvokePrecedesDequeueMidrunEvent.Kick, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: ""))
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_watch"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_watch", childSM, true, InvokePrecedesDequeueMidrunEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine
    override fun onExit(state: InvokePrecedesDequeueMidrunState) {
        when (state) {
            is InvokePrecedesDequeueMidrunState.Arm -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:45 :: arm :: _state_body
                activeStateIds.remove("arm")
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:89 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:88 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:54 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine
    override fun executeTransitionActions(
        source: InvokePrecedesDequeueMidrunState,
        event: InvokePrecedesDequeueMidrunEvent?
    ) {
        when (source) {
        is InvokePrecedesDequeueMidrunState.Phase -> when {
            event is InvokePrecedesDequeueMidrunEvent.Ready -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:82 :: phase :: _transition_1


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_watch", "probe")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
