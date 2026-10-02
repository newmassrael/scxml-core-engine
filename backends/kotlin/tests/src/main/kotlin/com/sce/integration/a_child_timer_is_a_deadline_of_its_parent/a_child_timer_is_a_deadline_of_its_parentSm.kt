// SCE-GENERATED — DO NOT EDIT
// source-hash: a3ca38872087c2314f9630b7be1b12a428c8ffa1560d49369c9f599f3caa573c

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:37 :: _machine

package com.sce.integration.a_child_timer_is_a_deadline_of_its_parent

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AChildTimerIsADeadlineOfItsParentState : State {
    data object Finished : AChildTimerIsADeadlineOfItsParentState
    data object Waiting : AChildTimerIsADeadlineOfItsParentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AChildTimerIsADeadlineOfItsParentEvent : Event {
    sealed interface Done : AChildTimerIsADeadlineOfItsParentEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Kid : Invoke
        }
    }
    sealed interface Error : AChildTimerIsADeadlineOfItsParentEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class AChildTimerIsADeadlineOfItsParentStateMachine(
) : StateMachineEngine<AChildTimerIsADeadlineOfItsParentState, AChildTimerIsADeadlineOfItsParentEvent>() {

    override val initialState: AChildTimerIsADeadlineOfItsParentState = AChildTimerIsADeadlineOfItsParentState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AChildTimerIsADeadlineOfItsParentState): Boolean = when (state) {
        is AChildTimerIsADeadlineOfItsParentState.Finished -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AChildTimerIsADeadlineOfItsParentState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AChildTimerIsADeadlineOfItsParentState, HistoryId>> =
            listOf(StateTarget(AChildTimerIsADeadlineOfItsParentState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<AChildTimerIsADeadlineOfItsParentState, HistoryId>(
            AChildTimerIsADeadlineOfItsParentState.Waiting,
            listOf(StateTarget(AChildTimerIsADeadlineOfItsParentState.Finished)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AChildTimerIsADeadlineOfItsParentState? = when (stateId) {
        "finished" -> AChildTimerIsADeadlineOfItsParentState.Finished
        "waiting" -> AChildTimerIsADeadlineOfItsParentState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AChildTimerIsADeadlineOfItsParentState): String = when (state) {
        is AChildTimerIsADeadlineOfItsParentState.Finished -> "finished"
        is AChildTimerIsADeadlineOfItsParentState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AChildTimerIsADeadlineOfItsParentState): Int = when (state) {
        is AChildTimerIsADeadlineOfItsParentState.Finished -> 1
        is AChildTimerIsADeadlineOfItsParentState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AChildTimerIsADeadlineOfItsParentEvent? = when (name) {
        "done.invoke" -> AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Self
        "done.invoke.kid" -> AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Kid
        "error.execution" -> AChildTimerIsADeadlineOfItsParentEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AChildTimerIsADeadlineOfItsParentEvent): String? = when (event) {
        is AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Self -> "done.invoke"
        is AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Kid -> "done.invoke.kid"
        is AChildTimerIsADeadlineOfItsParentEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AChildTimerIsADeadlineOfItsParentState,
        event: AChildTimerIsADeadlineOfItsParentEvent?
    ): EnabledTransition<AChildTimerIsADeadlineOfItsParentState, HistoryId>? = when (state) {
        is AChildTimerIsADeadlineOfItsParentState.Waiting -> when {
            event is AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Kid -> transitionWaitingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:37 :: _machine
    override fun onEntry(state: AChildTimerIsADeadlineOfItsParentState, isDefaultEntry: Boolean) {
        when (state) {
            is AChildTimerIsADeadlineOfItsParentState.Finished -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:65 :: finished :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AChildTimerIsADeadlineOfItsParentState.Waiting -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:41 :: waiting :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("kid", state)
            }
        }
    }

    // W3C SCXML 6.4: defer the start of the static child session of the
    // `<invoke type="scxml">` `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    private fun deferStaticInvoke(invokeId: String, state: AChildTimerIsADeadlineOfItsParentState) {
        when (invokeId) {
            "kid" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "waiting.${System.identityHashCode(this)}.kid"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidStateMachine()
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("kid", childSM, false, AChildTimerIsADeadlineOfItsParentEvent.Done.Invoke.Kid, "", generatedInvokeId)
                }
            }
            else -> error("the document has no static child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:37 :: _machine
    override fun onExit(state: AChildTimerIsADeadlineOfItsParentState) {
        when (state) {
            is AChildTimerIsADeadlineOfItsParentState.Finished -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:65 :: finished :: _state_body
            }
            is AChildTimerIsADeadlineOfItsParentState.Waiting -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:41 :: waiting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("kid")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent.scxml:37 :: _machine
    override fun executeTransitionContent(source: AChildTimerIsADeadlineOfItsParentState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
