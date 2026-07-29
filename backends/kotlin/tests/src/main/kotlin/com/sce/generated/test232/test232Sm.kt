// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367096

// GENERATED CODE — DO NOT EDIT
// Source: resources/232/test232.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test232.scxml:5

package com.sce.generated.test232

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test232State : State {
    data object Fail : Test232State
    data object Pass : Test232State
    data object S0 : Test232State
    data object S01 : Test232State
    data object S02 : Test232State
    data object S03 : Test232State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232Event : Event {
    sealed interface Cancel : Test232Event {
        data object Invoke : Cancel
    }
    data object ChildToParent1 : Test232Event
    data object ChildToParent2 : Test232Event
    sealed interface Done : Test232Event {
        data object Invoke : Done
    }
    sealed interface Error : Test232Event {
        data object Execution : Error
    }
    data object Timeout : Test232Event
}
// --- State Machine (W3C SCXML) ---

class Test232StateMachine(
) : StateMachineEngine<Test232State, Test232Event>() {

    override val initialState: Test232State = Test232State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test232State): Test232State? = when (state) {
        is Test232State.S01 -> Test232State.S0
        is Test232State.S02 -> Test232State.S0
        is Test232State.S03 -> Test232State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test232State): Test232State = when (state) {
        is Test232State.S0 -> Test232State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test232State? = when (stateId) {
        "fail" -> Test232State.Fail
        "pass" -> Test232State.Pass
        "s0" -> Test232State.S0
        "s01" -> Test232State.S01
        "s02" -> Test232State.S02
        "s03" -> Test232State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test232State): String = when (state) {
        is Test232State.Fail -> "fail"
        is Test232State.Pass -> "pass"
        is Test232State.S0 -> "s0"
        is Test232State.S01 -> "s01"
        is Test232State.S02 -> "s02"
        is Test232State.S03 -> "s03"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test232State): Boolean = when (state) {
        is Test232State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test232State): Int = when (state) {
        is Test232State.Fail -> 5
        is Test232State.Pass -> 4
        is Test232State.S0 -> 0
        is Test232State.S01 -> 1
        is Test232State.S02 -> 2
        is Test232State.S03 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232Event? = when (name) {
        "cancel.invoke" -> Test232Event.Cancel.Invoke
        "childToParent1" -> Test232Event.ChildToParent1
        "childToParent2" -> Test232Event.ChildToParent2
        "done.invoke" -> Test232Event.Done.Invoke
        "error.execution" -> Test232Event.Error.Execution
        "timeout" -> Test232Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232Event): String? = when (event) {
        is Test232Event.Cancel.Invoke -> "cancel.invoke"
        is Test232Event.ChildToParent1 -> "childToParent1"
        is Test232Event.ChildToParent2 -> "childToParent2"
        is Test232Event.Done.Invoke -> "done.invoke"
        is Test232Event.Error.Execution -> "error.execution"
        is Test232Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test232State,
        event: Test232Event
    ): TransitionResult<Test232State> = when (state) {
        is Test232State.S0 -> processS0(event)
        is Test232State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test232State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test232State.S03 -> {
            val result = processS03(event)
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
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.Timeout -> TransitionResult.External(Test232State.Fail, Test232State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.ChildToParent1 -> TransitionResult.External(Test232State.S02, Test232State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.ChildToParent2 -> TransitionResult.External(Test232State.S03, Test232State.S02)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test232Event
    ): TransitionResult<Test232State> = when {
        event is Test232Event.Done.Invoke -> TransitionResult.External(Test232State.Pass, Test232State.S03)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test232.scxml:5
    override fun onEntry(state: Test232State) {
        when (state) {
            is Test232State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test232State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test232Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test232SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test232Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test232State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test232State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test232State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test232.scxml:5
    override fun onExit(state: Test232State) {
        when (state) {
            is Test232State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test232State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test232State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test232State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test232State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test232State.S03 -> {
                activeStateIds.remove("s03")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test232.scxml:5
    override fun executeTransitionActions(
        source: Test232State,
        event: Test232Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
