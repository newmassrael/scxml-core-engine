// SCE-GENERATED — DO NOT EDIT
// source-hash: 7c010da1526dce3962148a99023f795b5efd3dc066529da8bc2dc12378934900

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml#inv_watch
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_external_dequeue.scxml:63 :: _machine

package com.sce.integration.invoke_precedes_external_dequeue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState : State {
    data object Missed : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Ordered : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Saw : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Waiting : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent : Event {
    sealed interface Error : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Kick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object Probe : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object Ready : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object SawKick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object SawNoKick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesExternalDequeueSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent>() {

    override val initialState: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState = InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): Boolean = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> true
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: saw's transition 0, as the microstep reads it.
        val transitionSawAt0 = EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw,
            listOf(StateTarget(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed
        "ordered" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered
        "saw" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw
        "waiting" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): String = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> "missed"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> "ordered"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> "saw"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): Int = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> 2
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> 3
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> 1
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent? = when (name) {
        "error.communication" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Communication
        "error.execution" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Execution
        "kick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick
        "probe" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe
        "ready" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Ready
        "sawKick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawKick
        "sawNoKick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawNoKick
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent): String? = when (event) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Communication -> "error.communication"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick -> "kick"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Ready -> "ready"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawKick -> "sawKick"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawNoKick -> "sawNoKick"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> firstEnabledTransitionOfSaw(state, event)
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // saw's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSaw(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>? = when {
        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> TransitionsPart0.transitionSawAt0
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, HistoryId>? = when {
        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick -> TransitionsPart0.transitionWaitingAt0
        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:63 :: _machine
    override fun onEntry(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> onEntryOfMissed()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> onEntryOfOrdered()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> onEntryOfSaw()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfMissed() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:79 :: missed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfOrdered() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:80 :: ordered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfSaw() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:74 :: saw :: _state_body
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:65 :: waiting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("ready", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:63 :: _machine
    override fun onExit(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> onExitOfMissed()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> onExitOfOrdered()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> onExitOfSaw()
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfMissed() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:79 :: missed :: _state_body
    }

    private fun onExitOfOrdered() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:80 :: ordered :: _state_body
    }

    private fun onExitOfSaw() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:74 :: saw :: _state_body
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:65 :: waiting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_external_dequeue.scxml:63 :: _machine
    override fun executeTransitionContent(source: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> transitionContentOfSaw(transitionIndex)
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> transitionContentOfWaiting(transitionIndex)
            else -> {}
        }
    }


    // The content of `saw`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSaw(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSawAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSawAt0() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:75 :: saw :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_2")
                return@send true
            }
            toParent("sawKick", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            1 -> transitionContentOfWaitingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt1() {
                // SCE-MAP: invoke_precedes_external_dequeue.scxml:70 :: waiting :: _transition_1


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("sawNoKick", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
