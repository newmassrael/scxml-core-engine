// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a
// template-hash: 9b6bfe76ab23aa9948245593703f14c85c86d24c4cb80ec29ba0173f5f4bb771
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42

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
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42
    override fun onEntry(state: InvokePrecedesDequeueMidrunState) {
        when (state) {
            is InvokePrecedesDequeueMidrunState.Arm -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("arm")) return


            send(InvokePrecedesDequeueMidrunEvent.Go, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
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
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42
    override fun onExit(state: InvokePrecedesDequeueMidrunState) {
        when (state) {
            is InvokePrecedesDequeueMidrunState.Arm -> {
                activeStateIds.remove("arm")
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                activeStateIds.remove("fail")
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                activeStateIds.remove("pass")
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42
    override fun executeTransitionActions(
        source: InvokePrecedesDequeueMidrunState,
        event: InvokePrecedesDequeueMidrunEvent?
    ) {
        when (source) {
        is InvokePrecedesDequeueMidrunState.Phase -> when {
            event is InvokePrecedesDequeueMidrunEvent.Ready -> {


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_watch", "probe")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
