// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun.scxml#inv_watch
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_dequeue_midrun.scxml:60 :: _machine

package com.sce.integration.invoke_precedes_dequeue_midrun

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState : State {
    data object Missed : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Ordered : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Saw : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Waiting : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent : Event {
    sealed interface Error : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Kick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object Probe : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object Ready : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object SawKick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object SawNoKick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent>() {

    override val initialState: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState = InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting

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
    override fun isFinalState(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): Boolean = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> true
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: saw's transition 0, as the microstep reads it.
        val transitionSawAt0 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed
        "ordered" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered
        "saw" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw
        "waiting" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): String = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> "missed"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> "ordered"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> "saw"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): Int = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> 2
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> 3
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> 1
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent? = when (name) {
        "error.communication" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Communication
        "error.execution" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Execution
        "kick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick
        "probe" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe
        "ready" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Ready
        "sawKick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawKick
        "sawNoKick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawNoKick
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent): String? = when (event) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Communication -> "error.communication"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick -> "kick"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Ready -> "ready"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawKick -> "sawKick"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawNoKick -> "sawNoKick"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> firstEnabledTransitionOfSaw(state, event)
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // saw's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSaw(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>? = when {
        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> TransitionsPart0.transitionSawAt0
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent?): EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>? = when {
        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick -> TransitionsPart0.transitionWaitingAt0
        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:60 :: _machine
    override fun onEntry(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> onEntryOfMissed()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> onEntryOfOrdered()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> onEntryOfSaw()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfMissed() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:76 :: missed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfOrdered() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:77 :: ordered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfSaw() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:71 :: saw :: _state_body
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:62 :: waiting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
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
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:60 :: _machine
    override fun onExit(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> onExitOfMissed()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> onExitOfOrdered()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> onExitOfSaw()
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfMissed() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:76 :: missed :: _state_body
    }

    private fun onExitOfOrdered() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:77 :: ordered :: _state_body
    }

    private fun onExitOfSaw() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:71 :: saw :: _state_body
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:62 :: waiting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:60 :: _machine
    override fun executeTransitionContent(source: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> transitionContentOfSaw(transitionIndex)
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> transitionContentOfWaiting(transitionIndex)
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
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:72 :: saw :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_2")
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
                // SCE-MAP: invoke_precedes_dequeue_midrun.scxml:67 :: waiting :: _transition_1


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
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
