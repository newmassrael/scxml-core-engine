// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/495/test495.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test495.scxml:4 :: _machine

package com.sce.generated.test495

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test495State : State {
    data object Fail : Test495State
    data object Pass : Test495State
    data object S0 : Test495State
    data object S1 : Test495State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test495Event : Event {
    data object Wildcard : Test495Event
    sealed interface Error : Test495Event {
        data object Execution : Error
    }
    data object Event1 : Test495Event
    data object Event2 : Test495Event
}
// --- State Machine (W3C SCXML) ---

class Test495StateMachine(
) : StateMachineEngine<Test495State, Test495Event>() {

    override val initialState: Test495State = Test495State.S0

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
    override fun isFinalState(state: Test495State): Boolean = when (state) {
        is Test495State.Fail -> true
        is Test495State.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test495State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test495State, HistoryId>> =
            listOf(StateTarget(Test495State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test495State, HistoryId>(
            Test495State.S0,
            listOf(StateTarget(Test495State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test495State, HistoryId>(
            Test495State.S0,
            listOf(StateTarget(Test495State.S1)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test495State, HistoryId>(
            Test495State.S1,
            listOf(StateTarget(Test495State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test495State, HistoryId>(
            Test495State.S1,
            listOf(StateTarget(Test495State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test495State? = when (stateId) {
        "fail" -> Test495State.Fail
        "pass" -> Test495State.Pass
        "s0" -> Test495State.S0
        "s1" -> Test495State.S1
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test495State): String = when (state) {
        is Test495State.Fail -> "fail"
        is Test495State.Pass -> "pass"
        is Test495State.S0 -> "s0"
        is Test495State.S1 -> "s1"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test495State): Int = when (state) {
        is Test495State.Fail -> 3
        is Test495State.Pass -> 2
        is Test495State.S0 -> 0
        is Test495State.S1 -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test495Event? = when (name) {
        "error.execution" -> Test495Event.Error.Execution
        "event1" -> Test495Event.Event1
        "event2" -> Test495Event.Event2
        "*" -> Test495Event.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test495Event): String? = when (event) {
        is Test495Event.Error.Execution -> "error.execution"
        is Test495Event.Event1 -> "event1"
        is Test495Event.Event2 -> "event2"
        is Test495Event.Wildcard -> "*"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test495State, event: Test495Event?): EnabledTransition<Test495State, HistoryId>? = when (state) {
        is Test495State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test495State.S1 -> firstEnabledTransitionOfS1(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test495State, event: Test495Event?): EnabledTransition<Test495State, HistoryId>? = when {
        event is Test495Event.Event1 -> TransitionsPart0.transitionS0At0
        event is Test495Event.Event2 -> TransitionsPart0.transitionS0At1
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test495State, event: Test495Event?): EnabledTransition<Test495State, HistoryId>? = when {
        event is Test495Event.Event1 -> TransitionsPart0.transitionS1At0
        event != null -> TransitionsPart0.transitionS1At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test495.scxml:4 :: _machine
    override fun onEntry(state: Test495State, isDefaultEntry: Boolean) {
        when (state) {
            is Test495State.Fail -> onEntryOfFail()
            is Test495State.Pass -> onEntryOfPass()
            is Test495State.S0 -> onEntryOfS0()
            is Test495State.S1 -> onEntryOfS1()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test495.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test495.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test495.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(Test495Event.Event1, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
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
            raiseInternal(Test495Event.Event2, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test495.scxml:18 :: s1 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test495.scxml:4 :: _machine
    override fun onExit(state: Test495State) {
        when (state) {
            is Test495State.Fail -> onExitOfFail()
            is Test495State.Pass -> onExitOfPass()
            is Test495State.S0 -> onExitOfS0()
            is Test495State.S1 -> onExitOfS1()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test495.scxml:24 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test495.scxml:23 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test495.scxml:7 :: s0 :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test495.scxml:18 :: s1 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test495.scxml:4 :: _machine
    override fun executeTransitionContent(source: Test495State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
