// SCE-GENERATED — DO NOT EDIT
// source-hash: ce55909c83cc4666c5ceb48ddcf2f5ce650a9da03007b3cc081cde9b3ac0761e
// template-hash: 4b3c3c02df8fbc8c8bdd14a46e1f1d9b76a9416609a553ce18199941c3392f19
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_dequeue_point.scxml:67

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
    sealed interface Cancel : AutoforwardDequeuePointEvent {
        data object Invoke : Cancel
    }
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardDequeuePointState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardDequeuePointState): Int = when (state) {
        is AutoforwardDequeuePointState.Fail -> 2
        is AutoforwardDequeuePointState.Pass -> 1
        is AutoforwardDequeuePointState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDequeuePointEvent? = when (name) {
        "cancel.invoke" -> AutoforwardDequeuePointEvent.Cancel.Invoke
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
        is AutoforwardDequeuePointEvent.Cancel.Invoke -> "cancel.invoke"
        is AutoforwardDequeuePointEvent.Done.Invoke -> "done.invoke"
        is AutoforwardDequeuePointEvent.Error.Execution -> "error.execution"
        is AutoforwardDequeuePointEvent.First -> "first"
        is AutoforwardDequeuePointEvent.Mark -> "mark"
        is AutoforwardDequeuePointEvent.Ready -> "ready"
        is AutoforwardDequeuePointEvent.SawMarkFirst -> "sawMarkFirst"
        is AutoforwardDequeuePointEvent.SawSecondEarly -> "sawSecondEarly"
        is AutoforwardDequeuePointEvent.Second -> "second"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardDequeuePointState,
        event: AutoforwardDequeuePointEvent
    ): TransitionResult<AutoforwardDequeuePointState> = when (state) {
        is AutoforwardDequeuePointState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: AutoforwardDequeuePointEvent
    ): TransitionResult<AutoforwardDequeuePointState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardDequeuePointEvent.Ready -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardDequeuePointEvent.First -> TransitionResult.Internal
        event is AutoforwardDequeuePointEvent.SawSecondEarly -> TransitionResult.External(AutoforwardDequeuePointState.Fail, AutoforwardDequeuePointState.Phase)

        event is AutoforwardDequeuePointEvent.SawMarkFirst -> TransitionResult.External(AutoforwardDequeuePointState.Pass, AutoforwardDequeuePointState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_dequeue_point.scxml:67
    override fun onEntry(state: AutoforwardDequeuePointState) {
        when (state) {
            is AutoforwardDequeuePointState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDequeuePointState.Phase -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
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
    // SCE-MAP: autoforward_dequeue_point.scxml:67
    override fun onExit(state: AutoforwardDequeuePointState) {
        when (state) {
            is AutoforwardDequeuePointState.Fail -> {
                activeStateIds.remove("fail")
            }
            is AutoforwardDequeuePointState.Pass -> {
                activeStateIds.remove("pass")
            }
            is AutoforwardDequeuePointState.Phase -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_probe")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_dequeue_point.scxml:67
    override fun executeTransitionActions(
        source: AutoforwardDequeuePointState,
        event: AutoforwardDequeuePointEvent?
    ) {
        when (source) {
        is AutoforwardDequeuePointState.Phase -> when {
            event is AutoforwardDequeuePointEvent.Ready -> {


            send(AutoforwardDequeuePointEvent.First, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            send(AutoforwardDequeuePointEvent.Second, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            event is AutoforwardDequeuePointEvent.First -> {


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_probe", "mark")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
