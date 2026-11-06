// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/416/test416.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test416.scxml:5 :: _machine

package com.sce.generated.test416

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test416State : State {
    data object Fail : Test416State
    data object Pass : Test416State
    data object S1 : Test416State
    data object S11 : Test416State
    data object S111 : Test416State
    data object S11final : Test416State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test416Event : Event {
    sealed interface Done : Test416Event {
        sealed interface State : Done {
            data object S11 : State
        }
    }
    sealed interface Error : Test416Event {
        data object Execution : Error
    }
    data object Timeout : Test416Event
}
// --- State Machine (W3C SCXML) ---

class Test416StateMachine(
) : StateMachineEngine<Test416State, Test416Event>() {

    override val initialState: Test416State = Test416State.S111

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test416State): Test416State? = when (state) {
        is Test416State.S11 -> Test416State.S1
        is Test416State.S111 -> Test416State.S11
        is Test416State.S11final -> Test416State.S11
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test416State): Boolean = when (state) {
        is Test416State.S1 -> true
        is Test416State.S11 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test416State): Boolean = when (state) {
        is Test416State.Fail -> true
        is Test416State.Pass -> true
        is Test416State.S11final -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test416State): List<Test416State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test416State): List<EntryTarget<Test416State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test416State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test416State, List<Test416State>> = LinkedHashMap<Test416State, List<Test416State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test416State, List<Test416State>>) {
            table[Test416State.S1] = listOf(Test416State.S11)
            table[Test416State.S11] = listOf(Test416State.S111, Test416State.S11final)
        }


        val initialTargets: Map<Test416State, List<EntryTarget<Test416State, HistoryId>>> = LinkedHashMap<Test416State, List<EntryTarget<Test416State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test416State, List<EntryTarget<Test416State, HistoryId>>>) {
            table[Test416State.S1] = listOf(StateTarget(Test416State.S11))
            table[Test416State.S11] = listOf(StateTarget(Test416State.S111))
        }


        val documentInitialTargetList: List<EntryTarget<Test416State, HistoryId>> =
            listOf(StateTarget(Test416State.S1))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test416State, HistoryId>(
            Test416State.S1,
            listOf(StateTarget(Test416State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11's transition 0, as the microstep reads it.
        val transitionS11At0 = EnabledTransition<Test416State, HistoryId>(
            Test416State.S11,
            listOf(StateTarget(Test416State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s111's transition 0, as the microstep reads it.
        val transitionS111At0 = EnabledTransition<Test416State, HistoryId>(
            Test416State.S111,
            listOf(StateTarget(Test416State.S11final)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test416State? = when (stateId) {
        "fail" -> Test416State.Fail
        "pass" -> Test416State.Pass
        "s1" -> Test416State.S1
        "s11" -> Test416State.S11
        "s111" -> Test416State.S111
        "s11final" -> Test416State.S11final
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test416State): String = when (state) {
        is Test416State.Fail -> "fail"
        is Test416State.Pass -> "pass"
        is Test416State.S1 -> "s1"
        is Test416State.S11 -> "s11"
        is Test416State.S111 -> "s111"
        is Test416State.S11final -> "s11final"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test416State): Int = when (state) {
        is Test416State.Fail -> 5
        is Test416State.Pass -> 4
        is Test416State.S1 -> 0
        is Test416State.S11 -> 1
        is Test416State.S111 -> 2
        is Test416State.S11final -> 3
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test416Event? = when (name) {
        "done.state.s11" -> Test416Event.Done.State.S11
        "error.execution" -> Test416Event.Error.Execution
        "timeout" -> Test416Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test416Event): String? = when (event) {
        is Test416Event.Done.State.S11 -> "done.state.s11"
        is Test416Event.Error.Execution -> "error.execution"
        is Test416Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test416State, event: Test416Event?): EnabledTransition<Test416State, HistoryId>? = when (state) {
        is Test416State.S1 -> firstEnabledTransitionOfS1(state, event)
        is Test416State.S11 -> firstEnabledTransitionOfS11(state, event)
        is Test416State.S111 -> firstEnabledTransitionOfS111(state, event)
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test416State, event: Test416Event?): EnabledTransition<Test416State, HistoryId>? = when {
        event is Test416Event.Timeout -> TransitionsPart0.transitionS1At0
        else -> null
    }


    // s11's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS11(state: Test416State, event: Test416Event?): EnabledTransition<Test416State, HistoryId>? = when {
        event is Test416Event.Done.State.S11 -> TransitionsPart0.transitionS11At0
        else -> null
    }


    // s111's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS111(state: Test416State, event: Test416Event?): EnabledTransition<Test416State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS111At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun onEntry(state: Test416State, isDefaultEntry: Boolean) {
        when (state) {
            is Test416State.Fail -> onEntryOfFail()
            is Test416State.Pass -> onEntryOfPass()
            is Test416State.S1 -> onEntryOfS1()
            is Test416State.S11 -> onEntryOfS11()
            is Test416State.S111 -> onEntryOfS111()
            is Test416State.S11final -> onEntryOfS11final()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test416.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test416.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test416.scxml:7 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 1000L, Test416Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS11() {
                // SCE-MAP: test416.scxml:13 :: s11 :: _state_body
    }

    private fun onEntryOfS111() {
                // SCE-MAP: test416.scxml:15 :: s111 :: _state_body
    }

    private fun onEntryOfS11final() {
                // SCE-MAP: test416.scxml:18 :: s11final :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test416Event.Done.State.S11, EventMetadata.platform())
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun onExit(state: Test416State) {
        when (state) {
            is Test416State.Fail -> onExitOfFail()
            is Test416State.Pass -> onExitOfPass()
            is Test416State.S1 -> onExitOfS1()
            is Test416State.S11 -> onExitOfS11()
            is Test416State.S111 -> onExitOfS111()
            is Test416State.S11final -> onExitOfS11final()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test416.scxml:24 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test416.scxml:23 :: pass :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test416.scxml:7 :: s1 :: _state_body
    }

    private fun onExitOfS11() {
                // SCE-MAP: test416.scxml:13 :: s11 :: _state_body
    }

    private fun onExitOfS111() {
                // SCE-MAP: test416.scxml:15 :: s111 :: _state_body
    }

    private fun onExitOfS11final() {
                // SCE-MAP: test416.scxml:18 :: s11final :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test416State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
