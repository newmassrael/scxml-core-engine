// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/247/test247.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test247.scxml:5 :: _machine

package com.sce.generated.test247

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test247State : State {
    data object Fail : Test247State
    data object Pass : Test247State
    data object S0 : Test247State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test247Event : Event {
    sealed interface Done : Test247Event {
        data object Invoke : Done
    }
    sealed interface Error : Test247Event {
        data object Execution : Error
    }
    data object Timeout : Test247Event
}
// --- State Machine (W3C SCXML) ---

class Test247StateMachine(
) : StateMachineEngine<Test247State, Test247Event>() {

    override val initialState: Test247State = Test247State.S0

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
    override fun isFinalState(state: Test247State): Boolean = when (state) {
        is Test247State.Fail -> true
        is Test247State.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test247State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test247State, HistoryId>> =
            listOf(StateTarget(Test247State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test247State, HistoryId>(
            Test247State.S0,
            listOf(StateTarget(Test247State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test247State, HistoryId>(
            Test247State.S0,
            listOf(StateTarget(Test247State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test247State? = when (stateId) {
        "fail" -> Test247State.Fail
        "pass" -> Test247State.Pass
        "s0" -> Test247State.S0
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test247State): String = when (state) {
        is Test247State.Fail -> "fail"
        is Test247State.Pass -> "pass"
        is Test247State.S0 -> "s0"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test247State): Int = when (state) {
        is Test247State.Fail -> 2
        is Test247State.Pass -> 1
        is Test247State.S0 -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test247Event? = when (name) {
        "done.invoke" -> Test247Event.Done.Invoke
        "error.execution" -> Test247Event.Error.Execution
        "timeout" -> Test247Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test247Event): String? = when (event) {
        is Test247Event.Done.Invoke -> "done.invoke"
        is Test247Event.Error.Execution -> "error.execution"
        is Test247Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test247State, event: Test247Event?): EnabledTransition<Test247State, HistoryId>? = when (state) {
        is Test247State.S0 -> firstEnabledTransitionOfS0(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test247State, event: Test247Event?): EnabledTransition<Test247State, HistoryId>? = when {
        event is Test247Event.Done.Invoke -> TransitionsPart0.transitionS0At0
        event is Test247Event.Timeout -> TransitionsPart0.transitionS0At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test247.scxml:5 :: _machine
    override fun onEntry(state: Test247State, isDefaultEntry: Boolean) {
        when (state) {
            is Test247State.Fail -> onEntryOfFail()
            is Test247State.Pass -> onEntryOfPass()
            is Test247State.S0 -> onEntryOfS0()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test247.scxml:25 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test247.scxml:24 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test247.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, Test247Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test247State.S0)
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
    private fun deferStaticInvoke(invokeId: String, state: Test247State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS0At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS0At0(state: Test247State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s0.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test247SceSynthInvokeInvoke0StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test247Event.Done.Invoke, "", generatedInvokeId, "", "test247__sce_synth_invoke__invoke_0", "_invoke_0")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test247.scxml:5 :: _machine
    override fun onExit(state: Test247State) {
        when (state) {
            is Test247State.Fail -> onExitOfFail()
            is Test247State.Pass -> onExitOfPass()
            is Test247State.S0 -> onExitOfS0()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test247.scxml:25 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test247.scxml:24 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test247.scxml:8 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test247State.S0)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test247.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test247State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
