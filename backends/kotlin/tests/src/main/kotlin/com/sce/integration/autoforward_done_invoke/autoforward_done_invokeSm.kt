// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b
// template-hash: e136547eba5b1b26d444df3b244f86733d75a97e370ef305f7a135f66e51e2c8
// generated-at: 0

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
    sealed interface Cancel : AutoforwardDoneInvokeEvent {
        data object Invoke : Cancel
    }
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardDoneInvokeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardDoneInvokeState): Int = when (state) {
        is AutoforwardDoneInvokeState.Fail -> 2
        is AutoforwardDoneInvokeState.Pass -> 1
        is AutoforwardDoneInvokeState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDoneInvokeEvent? = when (name) {
        "cancel.invoke" -> AutoforwardDoneInvokeEvent.Cancel.Invoke
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
        is AutoforwardDoneInvokeEvent.Cancel.Invoke -> "cancel.invoke"
        is AutoforwardDoneInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is AutoforwardDoneInvokeEvent.Done.Invoke.InvShort -> "done.invoke.inv_short"
        is AutoforwardDoneInvokeEvent.Error.Execution -> "error.execution"
        is AutoforwardDoneInvokeEvent.Probe -> "probe"
        is AutoforwardDoneInvokeEvent.SawPlatform -> "sawPlatform"
        is AutoforwardDoneInvokeEvent.SawProbeOnly -> "sawProbeOnly"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardDoneInvokeState,
        event: AutoforwardDoneInvokeEvent
    ): TransitionResult<AutoforwardDoneInvokeState> = when (state) {
        is AutoforwardDoneInvokeState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: AutoforwardDoneInvokeEvent
    ): TransitionResult<AutoforwardDoneInvokeState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardDoneInvokeEvent.Done.Invoke.InvShort -> TransitionResult.Internal
        event is AutoforwardDoneInvokeEvent.SawPlatform -> TransitionResult.External(AutoforwardDoneInvokeState.Pass, AutoforwardDoneInvokeState.Phase)

        event is AutoforwardDoneInvokeEvent.SawProbeOnly -> TransitionResult.External(AutoforwardDoneInvokeState.Fail, AutoforwardDoneInvokeState.Phase)

        event is AutoforwardDoneInvokeEvent.Error.Execution -> TransitionResult.External(AutoforwardDoneInvokeState.Fail, AutoforwardDoneInvokeState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeState) {
        when (state) {
            is AutoforwardDoneInvokeState.Fail -> {
                // SCE-MAP: autoforward_done_invoke.scxml:92 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeState.Pass -> {
                // SCE-MAP: autoforward_done_invoke.scxml:91 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeState.Phase -> {
                // SCE-MAP: autoforward_done_invoke.scxml:58 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
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
                activeStateIds.remove("fail")
            }
            is AutoforwardDoneInvokeState.Pass -> {
                // SCE-MAP: autoforward_done_invoke.scxml:91 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is AutoforwardDoneInvokeState.Phase -> {
                // SCE-MAP: autoforward_done_invoke.scxml:58 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_watch")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_short")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke.scxml:55 :: _machine
    override fun executeTransitionActions(
        source: AutoforwardDoneInvokeState,
        event: AutoforwardDoneInvokeEvent?
    ) {
        when (source) {
        is AutoforwardDoneInvokeState.Phase -> when {
            event is AutoforwardDoneInvokeEvent.Done.Invoke.InvShort -> {
                // SCE-MAP: autoforward_done_invoke.scxml:84 :: phase :: _transition_0


            send(AutoforwardDoneInvokeEvent.Probe, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
