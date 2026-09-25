// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c78d9a40e778435f5ba721a7a12bf6721453dde3c80246e5018de3fc670010

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
    override fun isFinalState(state: AutoforwardInternalQueueState): Boolean = when (state) {
        is AutoforwardInternalQueueState.Fail, is AutoforwardInternalQueueState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardInternalQueueState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardInternalQueueState, HistoryId>> =
            listOf(StateTarget(AutoforwardInternalQueueState.Phase))

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<AutoforwardInternalQueueState, HistoryId>(
            AutoforwardInternalQueueState.Phase,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 1, as the microstep reads it.
        val transitionPhaseAt1 = EnabledTransition<AutoforwardInternalQueueState, HistoryId>(
            AutoforwardInternalQueueState.Phase,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 2, as the microstep reads it.
        val transitionPhaseAt2 = EnabledTransition<AutoforwardInternalQueueState, HistoryId>(
            AutoforwardInternalQueueState.Phase,
            listOf(StateTarget(AutoforwardInternalQueueState.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 3, as the microstep reads it.
        val transitionPhaseAt3 = EnabledTransition<AutoforwardInternalQueueState, HistoryId>(
            AutoforwardInternalQueueState.Phase,
            listOf(StateTarget(AutoforwardInternalQueueState.Pass)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardInternalQueueState,
        event: AutoforwardInternalQueueEvent?
    ): EnabledTransition<AutoforwardInternalQueueState, HistoryId>? = when (state) {
        is AutoforwardInternalQueueState.Phase -> when {
            event is AutoforwardInternalQueueEvent.Ready -> transitionPhaseAt0
            event is AutoforwardInternalQueueEvent.Error.Execution -> transitionPhaseAt1
            event is AutoforwardInternalQueueEvent.SawInternal -> transitionPhaseAt2
            event is AutoforwardInternalQueueEvent.SawProbeOnly -> transitionPhaseAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine
    override fun onEntry(state: AutoforwardInternalQueueState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardInternalQueueState.Fail -> {
                // SCE-MAP: autoforward_internal_queue.scxml:85 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueState.Pass -> {
                // SCE-MAP: autoforward_internal_queue.scxml:84 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueState.Phase -> {
                // SCE-MAP: autoforward_internal_queue.scxml:54 :: phase :: _state_body
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
            }
            is AutoforwardInternalQueueState.Pass -> {
                // SCE-MAP: autoforward_internal_queue.scxml:84 :: pass :: _state_body
            }
            is AutoforwardInternalQueueState.Phase -> {
                // SCE-MAP: autoforward_internal_queue.scxml:54 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_internal_queue.scxml:51 :: _machine
    override fun executeTransitionContent(source: AutoforwardInternalQueueState, transitionIndex: Int) {
        when (source) {
        is AutoforwardInternalQueueState.Phase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_internal_queue.scxml:75 :: phase :: _transition_0


            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raisePlatformError(AutoforwardInternalQueueEvent.Error.Execution, "<send type='urn:x-sce-unsupported-processor'> names a processor this platform does not support", "__send_0")
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
            1 -> {
                // SCE-MAP: autoforward_internal_queue.scxml:78 :: phase :: _transition_1


            send(AutoforwardInternalQueueEvent.Probe, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
