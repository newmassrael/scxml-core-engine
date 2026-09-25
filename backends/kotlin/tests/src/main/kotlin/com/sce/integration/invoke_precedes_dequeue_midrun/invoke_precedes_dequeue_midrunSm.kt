// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a

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

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InvokePrecedesDequeueMidrunState): Boolean = when (state) {
        is InvokePrecedesDequeueMidrunState.Fail, is InvokePrecedesDequeueMidrunState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokePrecedesDequeueMidrunState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokePrecedesDequeueMidrunState, HistoryId>> =
            listOf(StateTarget(InvokePrecedesDequeueMidrunState.Arm))

        // W3C SCXML 3.13: arm's transition 0, as the microstep reads it.
        val transitionArmAt0 = EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>(
            InvokePrecedesDequeueMidrunState.Arm,
            listOf(StateTarget(InvokePrecedesDequeueMidrunState.Phase)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>(
            InvokePrecedesDequeueMidrunState.Phase,
            emptyList(),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 1, as the microstep reads it.
        val transitionPhaseAt1 = EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>(
            InvokePrecedesDequeueMidrunState.Phase,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 2, as the microstep reads it.
        val transitionPhaseAt2 = EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>(
            InvokePrecedesDequeueMidrunState.Phase,
            listOf(StateTarget(InvokePrecedesDequeueMidrunState.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 3, as the microstep reads it.
        val transitionPhaseAt3 = EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>(
            InvokePrecedesDequeueMidrunState.Phase,
            listOf(StateTarget(InvokePrecedesDequeueMidrunState.Fail)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: InvokePrecedesDequeueMidrunState,
        event: InvokePrecedesDequeueMidrunEvent?
    ): EnabledTransition<InvokePrecedesDequeueMidrunState, HistoryId>? = when (state) {
        is InvokePrecedesDequeueMidrunState.Arm -> when {
            event is InvokePrecedesDequeueMidrunEvent.Go -> transitionArmAt0
            else -> null
        }
        is InvokePrecedesDequeueMidrunState.Phase -> when {
            event is InvokePrecedesDequeueMidrunEvent.Kick -> transitionPhaseAt0
            event is InvokePrecedesDequeueMidrunEvent.Ready -> transitionPhaseAt1
            event is InvokePrecedesDequeueMidrunEvent.SawKick -> transitionPhaseAt2
            event is InvokePrecedesDequeueMidrunEvent.SawNoKick -> transitionPhaseAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine
    override fun onEntry(state: InvokePrecedesDequeueMidrunState, isDefaultEntry: Boolean) {
        when (state) {
            is InvokePrecedesDequeueMidrunState.Arm -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:45 :: arm :: _state_body


            send(InvokePrecedesDequeueMidrunEvent.Go, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:89 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:88 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:54 :: phase :: _state_body


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
            }
            is InvokePrecedesDequeueMidrunState.Fail -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:89 :: fail :: _state_body
            }
            is InvokePrecedesDequeueMidrunState.Pass -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:88 :: pass :: _state_body
            }
            is InvokePrecedesDequeueMidrunState.Phase -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:54 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:42 :: _machine
    override fun executeTransitionContent(source: InvokePrecedesDequeueMidrunState, transitionIndex: Int) {
        when (source) {
        is InvokePrecedesDequeueMidrunState.Phase -> when (transitionIndex) {
            1 -> {
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
