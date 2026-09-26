// SCE-GENERATED — DO NOT EDIT
// source-hash: ce55909c83cc4666c5ceb48ddcf2f5ce650a9da03007b3cc081cde9b3ac0761e

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_dequeue_point.scxml:67 :: _machine

package com.sce.integration.autoforward_dequeue_point

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDequeuePointState : State {
    data object Fail : AutoforwardDequeuePointState
    data object Pass : AutoforwardDequeuePointState
    data object Phase : AutoforwardDequeuePointState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDequeuePointEvent : Event {
    sealed interface Done : AutoforwardDequeuePointEvent {
        data object Invoke : Done
    }
    sealed interface Error : AutoforwardDequeuePointEvent {
        data object Execution : Error
    }
    data object First : AutoforwardDequeuePointEvent
    data object Mark : AutoforwardDequeuePointEvent
    data object Ready : AutoforwardDequeuePointEvent
    data object SawMarkFirst : AutoforwardDequeuePointEvent
    data object SawSecondEarly : AutoforwardDequeuePointEvent
    data object Second : AutoforwardDequeuePointEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardDequeuePointStateMachine(
) : StateMachineEngine<AutoforwardDequeuePointState, AutoforwardDequeuePointEvent>() {

    override val initialState: AutoforwardDequeuePointState = AutoforwardDequeuePointState.Phase

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
    override fun isFinalState(state: AutoforwardDequeuePointState): Boolean = when (state) {
        is AutoforwardDequeuePointState.Fail, is AutoforwardDequeuePointState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDequeuePointState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDequeuePointState, HistoryId>> =
            listOf(StateTarget(AutoforwardDequeuePointState.Phase))

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<AutoforwardDequeuePointState, HistoryId>(
            AutoforwardDequeuePointState.Phase,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 1, as the microstep reads it.
        val transitionPhaseAt1 = EnabledTransition<AutoforwardDequeuePointState, HistoryId>(
            AutoforwardDequeuePointState.Phase,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 2, as the microstep reads it.
        val transitionPhaseAt2 = EnabledTransition<AutoforwardDequeuePointState, HistoryId>(
            AutoforwardDequeuePointState.Phase,
            listOf(StateTarget(AutoforwardDequeuePointState.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 3, as the microstep reads it.
        val transitionPhaseAt3 = EnabledTransition<AutoforwardDequeuePointState, HistoryId>(
            AutoforwardDequeuePointState.Phase,
            listOf(StateTarget(AutoforwardDequeuePointState.Pass)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDequeuePointState? = when (stateId) {
        "fail" -> AutoforwardDequeuePointState.Fail
        "pass" -> AutoforwardDequeuePointState.Pass
        "phase" -> AutoforwardDequeuePointState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDequeuePointState): String = when (state) {
        is AutoforwardDequeuePointState.Fail -> "fail"
        is AutoforwardDequeuePointState.Pass -> "pass"
        is AutoforwardDequeuePointState.Phase -> "phase"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDequeuePointState): Int = when (state) {
        is AutoforwardDequeuePointState.Fail -> 2
        is AutoforwardDequeuePointState.Pass -> 1
        is AutoforwardDequeuePointState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDequeuePointEvent? = when (name) {
        "done.invoke" -> AutoforwardDequeuePointEvent.Done.Invoke
        "error.execution" -> AutoforwardDequeuePointEvent.Error.Execution
        "first" -> AutoforwardDequeuePointEvent.First
        "mark" -> AutoforwardDequeuePointEvent.Mark
        "ready" -> AutoforwardDequeuePointEvent.Ready
        "sawMarkFirst" -> AutoforwardDequeuePointEvent.SawMarkFirst
        "sawSecondEarly" -> AutoforwardDequeuePointEvent.SawSecondEarly
        "second" -> AutoforwardDequeuePointEvent.Second
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardDequeuePointEvent): String? = when (event) {
        is AutoforwardDequeuePointEvent.Done.Invoke -> "done.invoke"
        is AutoforwardDequeuePointEvent.Error.Execution -> "error.execution"
        is AutoforwardDequeuePointEvent.First -> "first"
        is AutoforwardDequeuePointEvent.Mark -> "mark"
        is AutoforwardDequeuePointEvent.Ready -> "ready"
        is AutoforwardDequeuePointEvent.SawMarkFirst -> "sawMarkFirst"
        is AutoforwardDequeuePointEvent.SawSecondEarly -> "sawSecondEarly"
        is AutoforwardDequeuePointEvent.Second -> "second"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardDequeuePointState,
        event: AutoforwardDequeuePointEvent?
    ): EnabledTransition<AutoforwardDequeuePointState, HistoryId>? = when (state) {
        is AutoforwardDequeuePointState.Phase -> when {
            event is AutoforwardDequeuePointEvent.Ready -> transitionPhaseAt0
            event is AutoforwardDequeuePointEvent.First -> transitionPhaseAt1
            event is AutoforwardDequeuePointEvent.SawSecondEarly -> transitionPhaseAt2
            event is AutoforwardDequeuePointEvent.SawMarkFirst -> transitionPhaseAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_dequeue_point.scxml:67 :: _machine
    override fun onEntry(state: AutoforwardDequeuePointState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDequeuePointState.Fail -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:108 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointState.Pass -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:107 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointState.Phase -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:70 :: phase :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_probe"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = AutoforwardDequeuePointSceSynthInvokeInvProbeStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_probe", childSM, true, AutoforwardDequeuePointEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_dequeue_point.scxml:67 :: _machine
    override fun onExit(state: AutoforwardDequeuePointState) {
        when (state) {
            is AutoforwardDequeuePointState.Fail -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:108 :: fail :: _state_body
            }
            is AutoforwardDequeuePointState.Pass -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:107 :: pass :: _state_body
            }
            is AutoforwardDequeuePointState.Phase -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:70 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_probe")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_dequeue_point.scxml:67 :: _machine
    override fun executeTransitionContent(source: AutoforwardDequeuePointState, transitionIndex: Int) {
        when (source) {
        is AutoforwardDequeuePointState.Phase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:97 :: phase :: _transition_0


            send(AutoforwardDequeuePointEvent.First, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            send(AutoforwardDequeuePointEvent.Second, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            1 -> {
                // SCE-MAP: autoforward_dequeue_point.scxml:101 :: phase :: _transition_1


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_probe", "mark")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
