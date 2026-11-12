// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml#inv_watch
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_done_invoke.scxml:61 :: _machine

package com.sce.integration.autoforward_done_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvWatchState : State {
    data object Missed : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
    data object Saw : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
    data object Watch : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent : Event {
    sealed interface Done : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent {
        sealed interface Invoke : Done {
            data object InvShort : Invoke
        }
    }
    sealed interface Error : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Probe : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    data object SawPlatform : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    data object SawProbeOnly : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardDoneInvokeSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent>() {

    override val initialState: AutoforwardDoneInvokeSceSynthInvokeInvWatchState = AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch

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
    override fun isFinalState(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): Boolean = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> true
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: watch's transition 0, as the microstep reads it.
        val transitionWatchAt0 = EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: watch's transition 1, as the microstep reads it.
        val transitionWatchAt1 = EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDoneInvokeSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed
        "saw" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw
        "watch" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): String = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> "missed"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> "saw"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> "watch"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): Int = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> 2
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> 1
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent? = when (name) {
        "done.invoke.inv_short" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort
        "error.communication" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Communication
        "error.execution" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Execution
        "probe" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe
        "sawPlatform" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawPlatform
        "sawProbeOnly" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawProbeOnly
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent): String? = when (event) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> "done.invoke.inv_short"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Communication -> "error.communication"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawPlatform -> "sawPlatform"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawProbeOnly -> "sawProbeOnly"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent?): EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> firstEnabledTransitionOfWatch(state, event)
        else -> null
    }


    // watch's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWatch(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent?): EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>? = when {
        event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> TransitionsPart0.transitionWatchAt0
        event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> TransitionsPart0.transitionWatchAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke.scxml:61 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> onEntryOfMissed()
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> onEntryOfSaw()
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> onEntryOfWatch()
        }
    }


    private fun onEntryOfMissed() {
                // SCE-MAP: autoforward_done_invoke.scxml:72 :: missed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfSaw() {
                // SCE-MAP: autoforward_done_invoke.scxml:71 :: saw :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWatch() {
                // SCE-MAP: autoforward_done_invoke.scxml:63 :: watch :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke.scxml:61 :: _machine
    override fun onExit(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> onExitOfMissed()
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> onExitOfSaw()
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> onExitOfWatch()
        }
    }


    private fun onExitOfMissed() {
                // SCE-MAP: autoforward_done_invoke.scxml:72 :: missed :: _state_body
    }

    private fun onExitOfSaw() {
                // SCE-MAP: autoforward_done_invoke.scxml:71 :: saw :: _state_body
    }

    private fun onExitOfWatch() {
                // SCE-MAP: autoforward_done_invoke.scxml:63 :: watch :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke.scxml:61 :: _machine
    override fun executeTransitionContent(source: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> transitionContentOfWatch(transitionIndex)
            else -> {}
        }
    }


    // The content of `watch`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWatch(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWatchAt0()
            1 -> transitionContentOfWatchAt1()
            else -> {}
        }
    }


    private fun transitionContentOfWatchAt0() {
                // SCE-MAP: autoforward_done_invoke.scxml:64 :: watch :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("sawPlatform", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfWatchAt1() {
                // SCE-MAP: autoforward_done_invoke.scxml:67 :: watch :: _transition_1


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("sawProbeOnly", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
