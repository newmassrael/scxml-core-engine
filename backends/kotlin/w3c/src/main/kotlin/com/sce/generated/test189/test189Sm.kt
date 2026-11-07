// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/189/test189.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test189.scxml:6 :: _machine

package com.sce.generated.test189

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test189State : State {
    data object Fail : Test189State
    data object Pass : Test189State
    data object S0 : Test189State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test189Event : Event {
    sealed interface Error : Test189Event {
        data object Execution : Error
    }
    data object Event1 : Test189Event
    data object Event2 : Test189Event
}
// --- State Machine (W3C SCXML) ---

class Test189StateMachine(
) : StateMachineEngine<Test189State, Test189Event>() {

    override val initialState: Test189State = Test189State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test189State): Boolean = when (state) {
        is Test189State.Fail -> true
        is Test189State.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test189State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test189State, HistoryId>> =
            listOf(StateTarget(Test189State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test189State, HistoryId>(
            Test189State.S0,
            listOf(StateTarget(Test189State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test189State, HistoryId>(
            Test189State.S0,
            listOf(StateTarget(Test189State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test189State? = when (stateId) {
        "fail" -> Test189State.Fail
        "pass" -> Test189State.Pass
        "s0" -> Test189State.S0
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test189State): String = when (state) {
        is Test189State.Fail -> "fail"
        is Test189State.Pass -> "pass"
        is Test189State.S0 -> "s0"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test189State): Int = when (state) {
        is Test189State.Fail -> 2
        is Test189State.Pass -> 1
        is Test189State.S0 -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test189Event? = when (name) {
        "error.execution" -> Test189Event.Error.Execution
        "event1" -> Test189Event.Event1
        "event2" -> Test189Event.Event2
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test189Event): String? = when (event) {
        is Test189Event.Error.Execution -> "error.execution"
        is Test189Event.Event1 -> "event1"
        is Test189Event.Event2 -> "event2"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test189State, event: Test189Event?): EnabledTransition<Test189State, HistoryId>? = when (state) {
        is Test189State.S0 -> firstEnabledTransitionOfS0(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test189State, event: Test189Event?): EnabledTransition<Test189State, HistoryId>? = when {
        event is Test189Event.Event1 -> TransitionsPart0.transitionS0At0
        event is Test189Event.Event2 -> TransitionsPart0.transitionS0At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun onEntry(state: Test189State, isDefaultEntry: Boolean) {
        when (state) {
            is Test189State.Fail -> onEntryOfFail()
            is Test189State.Pass -> onEntryOfPass()
            is Test189State.S0 -> onEntryOfS0()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test189.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test189.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test189.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(Test189Event.Event2, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(Test189Event.Event1, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun onExit(state: Test189State) {
        when (state) {
            is Test189State.Fail -> onExitOfFail()
            is Test189State.Pass -> onExitOfPass()
            is Test189State.S0 -> onExitOfS0()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test189.scxml:23 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test189.scxml:22 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test189.scxml:9 :: s0 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test189State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
