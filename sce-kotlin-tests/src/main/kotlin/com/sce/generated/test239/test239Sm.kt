// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486

// GENERATED CODE — DO NOT EDIT
// Source: resources/239/test239.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test239.scxml:5

package com.sce.generated.test239

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test239State : State {
    data object Fail : Test239State
    data object Pass : Test239State
    data object S0 : Test239State
    data object S01 : Test239State
    data object S02 : Test239State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test239Event : Event {
    sealed interface Cancel : Test239Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test239Event {
        data object Invoke : Done
    }
    sealed interface Error : Test239Event {
        data object Execution : Error
    }
    data object Timeout : Test239Event
}
// --- State Machine (W3C SCXML) ---

class Test239StateMachine(
) : StateMachineEngine<Test239State, Test239Event>() {

    override val initialState: Test239State = Test239State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test239State): Test239State? = when (state) {
        is Test239State.S01 -> Test239State.S0
        is Test239State.S02 -> Test239State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test239State): Test239State = when (state) {
        is Test239State.S0 -> Test239State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test239State? = when (stateId) {
        "fail" -> Test239State.Fail
        "pass" -> Test239State.Pass
        "s0" -> Test239State.S0
        "s01" -> Test239State.S01
        "s02" -> Test239State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test239State): String = when (state) {
        is Test239State.Fail -> "fail"
        is Test239State.Pass -> "pass"
        is Test239State.S0 -> "s0"
        is Test239State.S01 -> "s01"
        is Test239State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test239State): Boolean = when (state) {
        is Test239State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test239State): Int = when (state) {
        is Test239State.Fail -> 4
        is Test239State.Pass -> 3
        is Test239State.S0 -> 0
        is Test239State.S01 -> 1
        is Test239State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test239Event? = when (name) {
        "cancel.invoke" -> Test239Event.Cancel.Invoke
        "done.invoke" -> Test239Event.Done.Invoke
        "error.execution" -> Test239Event.Error.Execution
        "timeout" -> Test239Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test239Event): String? = when (event) {
        is Test239Event.Cancel.Invoke -> "cancel.invoke"
        is Test239Event.Done.Invoke -> "done.invoke"
        is Test239Event.Error.Execution -> "error.execution"
        is Test239Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test239State,
        event: Test239Event
    ): TransitionResult<Test239State> = when (state) {
        is Test239State.S0 -> processS0(event)
        is Test239State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test239State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test239Event
    ): TransitionResult<Test239State> = when {
        event is Test239Event.Timeout -> TransitionResult.External(Test239State.Fail, Test239State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test239Event
    ): TransitionResult<Test239State> = when {
        event is Test239Event.Done.Invoke -> TransitionResult.External(Test239State.S02, Test239State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test239Event
    ): TransitionResult<Test239State> = when {
        event is Test239Event.Done.Invoke -> TransitionResult.External(Test239State.Pass, Test239State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test239.scxml:5
    override fun onEntry(state: Test239State) {
        when (state) {
            is Test239State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test239State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test239State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test239Event.Timeout)
            }
            is Test239State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test239sub1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test239Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test239State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test239SceSynthInvokeInvoke1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test239Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test239.scxml:5
    override fun onExit(state: Test239State) {
        when (state) {
            is Test239State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test239State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test239State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test239State.S01 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s01")
            }
            is Test239State.S02 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test239.scxml:5
    override fun executeTransitionActions(
        source: Test239State,
        event: Test239Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
