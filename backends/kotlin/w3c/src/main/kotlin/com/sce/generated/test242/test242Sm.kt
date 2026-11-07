// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242.scxml:6 :: _machine

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242State : State {
    data object Fail : Test242State
    data object Pass : Test242State
    data object S0 : Test242State
    data object S02 : Test242State
    data object S03 : Test242State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242Event : Event {
    sealed interface Done : Test242Event {
        data object Invoke : Done
    }
    sealed interface Error : Test242Event {
        data object Execution : Error
    }
    data object Timeout : Test242Event
    data object Timeout1 : Test242Event
    data object Timeout2 : Test242Event
    data object Timeout3 : Test242Event
}
// --- State Machine (W3C SCXML) ---

class Test242StateMachine(
) : StateMachineEngine<Test242State, Test242Event>() {

    override val initialState: Test242State = Test242State.S0

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
    override fun isFinalState(state: Test242State): Boolean = when (state) {
        is Test242State.Fail -> true
        is Test242State.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test242State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test242State, HistoryId>> =
            listOf(StateTarget(Test242State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.S02)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S0,
            listOf(StateTarget(Test242State.S03)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S02,
            listOf(StateTarget(Test242State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S02,
            listOf(StateTarget(Test242State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S03,
            listOf(StateTarget(Test242State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test242State, HistoryId>(
            Test242State.S03,
            listOf(StateTarget(Test242State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242State? = when (stateId) {
        "fail" -> Test242State.Fail
        "pass" -> Test242State.Pass
        "s0" -> Test242State.S0
        "s02" -> Test242State.S02
        "s03" -> Test242State.S03
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242State): String = when (state) {
        is Test242State.Fail -> "fail"
        is Test242State.Pass -> "pass"
        is Test242State.S0 -> "s0"
        is Test242State.S02 -> "s02"
        is Test242State.S03 -> "s03"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test242State): Int = when (state) {
        is Test242State.Fail -> 4
        is Test242State.Pass -> 3
        is Test242State.S0 -> 0
        is Test242State.S02 -> 1
        is Test242State.S03 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test242Event? = when (name) {
        "done.invoke" -> Test242Event.Done.Invoke
        "error.execution" -> Test242Event.Error.Execution
        "timeout" -> Test242Event.Timeout
        "timeout1" -> Test242Event.Timeout1
        "timeout2" -> Test242Event.Timeout2
        "timeout3" -> Test242Event.Timeout3
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test242Event): String? = when (event) {
        is Test242Event.Done.Invoke -> "done.invoke"
        is Test242Event.Error.Execution -> "error.execution"
        is Test242Event.Timeout -> "timeout"
        is Test242Event.Timeout1 -> "timeout1"
        is Test242Event.Timeout2 -> "timeout2"
        is Test242Event.Timeout3 -> "timeout3"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test242State, event: Test242Event?): EnabledTransition<Test242State, HistoryId>? = when (state) {
        is Test242State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test242State.S02 -> firstEnabledTransitionOfS02(state, event)
        is Test242State.S03 -> firstEnabledTransitionOfS03(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test242State, event: Test242Event?): EnabledTransition<Test242State, HistoryId>? = when {
        event is Test242Event.Timeout -> TransitionsPart0.transitionS0At0
        event is Test242Event.Done.Invoke -> TransitionsPart0.transitionS0At1
        event is Test242Event.Timeout1 -> TransitionsPart0.transitionS0At2
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test242State, event: Test242Event?): EnabledTransition<Test242State, HistoryId>? = when {
        event is Test242Event.Done.Invoke -> TransitionsPart0.transitionS02At0
        event is Test242Event.Timeout2 -> TransitionsPart0.transitionS02At1
        else -> null
    }


    // s03's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS03(state: Test242State, event: Test242Event?): EnabledTransition<Test242State, HistoryId>? = when {
        event is Test242Event.Timeout3 -> TransitionsPart0.transitionS03At0
        event is Test242Event.Done.Invoke -> TransitionsPart0.transitionS03At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun onEntry(state: Test242State, isDefaultEntry: Boolean) {
        when (state) {
            is Test242State.Fail -> onEntryOfFail()
            is Test242State.Pass -> onEntryOfPass()
            is Test242State.S0 -> onEntryOfS0()
            is Test242State.S02 -> onEntryOfS02()
            is Test242State.S03 -> onEntryOfS03()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test242.scxml:56 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test242.scxml:55 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test242.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 1000L, Test242Event.Timeout1, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test242State.S0)
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test242.scxml:20 :: s02 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_1", 1000L, Test242Event.Timeout2, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_1", Test242State.S02)
    }

    private fun onEntryOfS03() {
                // SCE-MAP: test242.scxml:37 :: s03 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", 1000L, Test242Event.Timeout3, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_2", Test242State.S03)
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    //
    // Each invoke's start is a method of its own (`startInvokeOf<State>At<n>`,
    // `n` its place among its state's invokes), and this dispatches to it with
    // one call an arm, for the reason `onEntry` does.
    private fun deferStaticInvoke(invokeId: String, state: Test242State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS0At0(state)
            "_invoke_1" -> startInvokeOfS02At0(state)
            "_invoke_2" -> startInvokeOfS03At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS0At0(state: Test242State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s0.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test242sub1StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId, "test242sub1", "test242sub1", "_invoke_0")
                }
            }
    }

    private fun startInvokeOfS02At0(state: Test242State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s02.${nextInvokeStart("_invoke_1")}._invoke_1"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test242SceSynthInvokeInvoke1StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_1", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId, "", "test242__sce_synth_invoke__invoke_1", "_invoke_1")
                }
            }
    }

    private fun startInvokeOfS03At0(state: Test242State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s03.${nextInvokeStart("_invoke_2")}._invoke_2"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test242SceSynthInvokeInvoke2StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_2", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId, "", "test242__sce_synth_invoke__invoke_2", "_invoke_2")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun onExit(state: Test242State) {
        when (state) {
            is Test242State.Fail -> onExitOfFail()
            is Test242State.Pass -> onExitOfPass()
            is Test242State.S0 -> onExitOfS0()
            is Test242State.S02 -> onExitOfS02()
            is Test242State.S03 -> onExitOfS03()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test242.scxml:56 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test242.scxml:55 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test242.scxml:9 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test242State.S0)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS02() {
                // SCE-MAP: test242.scxml:20 :: s02 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test242State.S02)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
    }

    private fun onExitOfS03() {
                // SCE-MAP: test242.scxml:37 :: s03 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test242State.S03)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test242.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test242State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
