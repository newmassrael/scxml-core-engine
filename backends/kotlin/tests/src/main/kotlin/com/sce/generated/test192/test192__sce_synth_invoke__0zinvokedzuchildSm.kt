// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:3 :: _machine

package com.sce.generated.test192

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test192SceSynthInvoke0zinvokedzuchildState : State {
    data object Sub0 : Test192SceSynthInvoke0zinvokedzuchildState
    data object SubFinal : Test192SceSynthInvoke0zinvokedzuchildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test192SceSynthInvoke0zinvokedzuchildEvent : Event {
    data object ChildToParent : Test192SceSynthInvoke0zinvokedzuchildEvent
    sealed interface Error : Test192SceSynthInvoke0zinvokedzuchildEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object EventReceived : Test192SceSynthInvoke0zinvokedzuchildEvent
    data object ParentToChild : Test192SceSynthInvoke0zinvokedzuchildEvent
    data object Timeout : Test192SceSynthInvoke0zinvokedzuchildEvent
}
// --- State Machine (W3C SCXML) ---

class Test192SceSynthInvoke0zinvokedzuchildStateMachine(
) : StateMachineEngine<Test192SceSynthInvoke0zinvokedzuchildState, Test192SceSynthInvoke0zinvokedzuchildEvent>() {

    override val initialState: Test192SceSynthInvoke0zinvokedzuchildState = Test192SceSynthInvoke0zinvokedzuchildState.Sub0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test192SceSynthInvoke0zinvokedzuchildState): Boolean = when (state) {
        is Test192SceSynthInvoke0zinvokedzuchildState.SubFinal -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test192SceSynthInvoke0zinvokedzuchildState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test192SceSynthInvoke0zinvokedzuchildState, HistoryId>> =
            listOf(StateTarget(Test192SceSynthInvoke0zinvokedzuchildState.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test192SceSynthInvoke0zinvokedzuchildState, HistoryId>(
            Test192SceSynthInvoke0zinvokedzuchildState.Sub0,
            listOf(StateTarget(Test192SceSynthInvoke0zinvokedzuchildState.SubFinal)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: sub0's transition 1, as the microstep reads it.
        val transitionSub0At1 = EnabledTransition<Test192SceSynthInvoke0zinvokedzuchildState, HistoryId>(
            Test192SceSynthInvoke0zinvokedzuchildState.Sub0,
            listOf(StateTarget(Test192SceSynthInvoke0zinvokedzuchildState.SubFinal)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test192SceSynthInvoke0zinvokedzuchildState? = when (stateId) {
        "sub0" -> Test192SceSynthInvoke0zinvokedzuchildState.Sub0
        "subFinal" -> Test192SceSynthInvoke0zinvokedzuchildState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test192SceSynthInvoke0zinvokedzuchildState): String = when (state) {
        is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> "sub0"
        is Test192SceSynthInvoke0zinvokedzuchildState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test192SceSynthInvoke0zinvokedzuchildState): Int = when (state) {
        is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> 0
        is Test192SceSynthInvoke0zinvokedzuchildState.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test192SceSynthInvoke0zinvokedzuchildEvent? = when (name) {
        "childToParent" -> Test192SceSynthInvoke0zinvokedzuchildEvent.ChildToParent
        "error.communication" -> Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Communication
        "error.execution" -> Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Execution
        "eventReceived" -> Test192SceSynthInvoke0zinvokedzuchildEvent.EventReceived
        "parentToChild" -> Test192SceSynthInvoke0zinvokedzuchildEvent.ParentToChild
        "timeout" -> Test192SceSynthInvoke0zinvokedzuchildEvent.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test192SceSynthInvoke0zinvokedzuchildEvent): String? = when (event) {
        is Test192SceSynthInvoke0zinvokedzuchildEvent.ChildToParent -> "childToParent"
        is Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Communication -> "error.communication"
        is Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Execution -> "error.execution"
        is Test192SceSynthInvoke0zinvokedzuchildEvent.EventReceived -> "eventReceived"
        is Test192SceSynthInvoke0zinvokedzuchildEvent.ParentToChild -> "parentToChild"
        is Test192SceSynthInvoke0zinvokedzuchildEvent.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test192SceSynthInvoke0zinvokedzuchildState,
        event: Test192SceSynthInvoke0zinvokedzuchildEvent?
    ): EnabledTransition<Test192SceSynthInvoke0zinvokedzuchildState, HistoryId>? = when (state) {
        is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> when {
            event is Test192SceSynthInvoke0zinvokedzuchildEvent.ParentToChild -> transitionSub0At0
            event is Test192SceSynthInvoke0zinvokedzuchildEvent.Timeout -> transitionSub0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:3 :: _machine
    override fun onEntry(state: Test192SceSynthInvoke0zinvokedzuchildState, isDefaultEntry: Boolean) {
        when (state) {
            is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> {
                // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:5 :: sub0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("childToParent", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", 3000L, Test192SceSynthInvoke0zinvokedzuchildEvent.Timeout, EventMetadata.external(sendId = "__send_2", origin = sessionId, data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is Test192SceSynthInvoke0zinvokedzuchildState.SubFinal -> {
                // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:18 :: subFinal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:3 :: _machine
    override fun onExit(state: Test192SceSynthInvoke0zinvokedzuchildState) {
        when (state) {
            is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> {
                // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:5 :: sub0 :: _state_body
            }
            is Test192SceSynthInvoke0zinvokedzuchildState.SubFinal -> {
                // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:18 :: subFinal :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test192SceSynthInvoke0zinvokedzuchildState, transitionIndex: Int) {
        when (source) {
        is Test192SceSynthInvoke0zinvokedzuchildState.Sub0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: test192__sce_synth_invoke__0zinvokedzuchild.scxml:11 :: sub0 :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(Test192SceSynthInvoke0zinvokedzuchildEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("eventReceived", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
