// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine

package com.sce.integration.autoforward_done_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDoneInvokeState : State {
    data object Fail : AutoforwardDoneInvokeState
    data object Pass : AutoforwardDoneInvokeState
    data object Phase : AutoforwardDoneInvokeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDoneInvokeEvent : Event {
    sealed interface Done : AutoforwardDoneInvokeEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object InvShort : Invoke
        }
    }
    sealed interface Error : AutoforwardDoneInvokeEvent {
        data object Execution : Error
    }
    data object Probe : AutoforwardDoneInvokeEvent
    data object SawPlatform : AutoforwardDoneInvokeEvent
    data object SawProbeOnly : AutoforwardDoneInvokeEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardDoneInvokeStateMachine(
) : StateMachineEngine<AutoforwardDoneInvokeState, AutoforwardDoneInvokeEvent>() {

    override val initialState: AutoforwardDoneInvokeState = AutoforwardDoneInvokeState.Phase

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
    override fun isFinalState(state: AutoforwardDoneInvokeState): Boolean = when (state) {
        is AutoforwardDoneInvokeState.Fail, is AutoforwardDoneInvokeState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDoneInvokeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDoneInvokeState, HistoryId>> =
            listOf(StateTarget(AutoforwardDoneInvokeState.Phase))

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<AutoforwardDoneInvokeState, HistoryId>(
            AutoforwardDoneInvokeState.Phase,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 1, as the microstep reads it.
        val transitionPhaseAt1 = EnabledTransition<AutoforwardDoneInvokeState, HistoryId>(
            AutoforwardDoneInvokeState.Phase,
            listOf(StateTarget(AutoforwardDoneInvokeState.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 2, as the microstep reads it.
        val transitionPhaseAt2 = EnabledTransition<AutoforwardDoneInvokeState, HistoryId>(
            AutoforwardDoneInvokeState.Phase,
            listOf(StateTarget(AutoforwardDoneInvokeState.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 3, as the microstep reads it.
        val transitionPhaseAt3 = EnabledTransition<AutoforwardDoneInvokeState, HistoryId>(
            AutoforwardDoneInvokeState.Phase,
            listOf(StateTarget(AutoforwardDoneInvokeState.Fail)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDoneInvokeState? = when (stateId) {
        "fail" -> AutoforwardDoneInvokeState.Fail
        "pass" -> AutoforwardDoneInvokeState.Pass
        "phase" -> AutoforwardDoneInvokeState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDoneInvokeState): String = when (state) {
        is AutoforwardDoneInvokeState.Fail -> "fail"
        is AutoforwardDoneInvokeState.Pass -> "pass"
        is AutoforwardDoneInvokeState.Phase -> "phase"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDoneInvokeState): Int = when (state) {
        is AutoforwardDoneInvokeState.Fail -> 2
        is AutoforwardDoneInvokeState.Pass -> 1
        is AutoforwardDoneInvokeState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDoneInvokeEvent? = when (name) {
        "done.invoke" -> AutoforwardDoneInvokeEvent.Done.Invoke.Self
        "done.invoke.inv_short" -> AutoforwardDoneInvokeEvent.Done.Invoke.InvShort
        "error.execution" -> AutoforwardDoneInvokeEvent.Error.Execution
        "probe" -> AutoforwardDoneInvokeEvent.Probe
        "sawPlatform" -> AutoforwardDoneInvokeEvent.SawPlatform
        "sawProbeOnly" -> AutoforwardDoneInvokeEvent.SawProbeOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardDoneInvokeEvent): String? = when (event) {
        is AutoforwardDoneInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is AutoforwardDoneInvokeEvent.Done.Invoke.InvShort -> "done.invoke.inv_short"
        is AutoforwardDoneInvokeEvent.Error.Execution -> "error.execution"
        is AutoforwardDoneInvokeEvent.Probe -> "probe"
        is AutoforwardDoneInvokeEvent.SawPlatform -> "sawPlatform"
        is AutoforwardDoneInvokeEvent.SawProbeOnly -> "sawProbeOnly"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardDoneInvokeState,
        event: AutoforwardDoneInvokeEvent?
    ): EnabledTransition<AutoforwardDoneInvokeState, HistoryId>? = when (state) {
        is AutoforwardDoneInvokeState.Phase -> when {
            event is AutoforwardDoneInvokeEvent.Done.Invoke.InvShort -> transitionPhaseAt0
            event is AutoforwardDoneInvokeEvent.SawPlatform -> transitionPhaseAt1
            event is AutoforwardDoneInvokeEvent.SawProbeOnly -> transitionPhaseAt2
            event is AutoforwardDoneInvokeEvent.Error.Execution -> transitionPhaseAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDoneInvokeState.Fail -> {
                // SCE-MAP: autoforward_done_invoke.scxml:92 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeState.Pass -> {
                // SCE-MAP: autoforward_done_invoke.scxml:91 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeState.Phase -> {
                // SCE-MAP: autoforward_done_invoke.scxml:58 :: phase :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_watch"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = AutoforwardDoneInvokeSceSynthInvokeInvWatchStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_watch", childSM, true, AutoforwardDoneInvokeEvent.Done.Invoke.Self, "", generatedInvokeId)
                    }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_short"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = AutoforwardDoneInvokeSceSynthInvokeInvShortStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_short", childSM, false, AutoforwardDoneInvokeEvent.Done.Invoke.InvShort, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine
    override fun onExit(state: AutoforwardDoneInvokeState) {
        when (state) {
            is AutoforwardDoneInvokeState.Fail -> {
                // SCE-MAP: autoforward_done_invoke.scxml:92 :: fail :: _state_body
            }
            is AutoforwardDoneInvokeState.Pass -> {
                // SCE-MAP: autoforward_done_invoke.scxml:91 :: pass :: _state_body
            }
            is AutoforwardDoneInvokeState.Phase -> {
                // SCE-MAP: autoforward_done_invoke.scxml:58 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_short")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine
    override fun executeTransitionContent(source: AutoforwardDoneInvokeState, transitionIndex: Int) {
        when (source) {
        is AutoforwardDoneInvokeState.Phase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_done_invoke.scxml:84 :: phase :: _transition_0


            send(AutoforwardDoneInvokeEvent.Probe, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
