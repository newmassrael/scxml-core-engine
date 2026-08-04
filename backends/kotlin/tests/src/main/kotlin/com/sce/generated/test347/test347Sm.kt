// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 817d9c061804919d9748138703a11f334a156a4e2a1e5a3c66f1c4e7ca554aa2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/347/test347.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test347.scxml:6

package com.sce.generated.test347

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test347State : State {
    data object Fail : Test347State
    data object Pass : Test347State
    data object S0 : Test347State
    data object S01 : Test347State
    data object S02 : Test347State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test347Event : Event {
    sealed interface Cancel : Test347Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test347Event
    sealed interface Done : Test347Event {
        data object Invoke : Done
    }
    sealed interface Error : Test347Event {
        data object Self : Error
        data object Execution : Error
    }
    data object ParentToChild : Test347Event
    data object Timeout : Test347Event
}
// --- State Machine (W3C SCXML) ---

class Test347StateMachine(
) : StateMachineEngine<Test347State, Test347Event>() {

    override val initialState: Test347State = Test347State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test347State): Test347State? = when (state) {
        is Test347State.S01 -> Test347State.S0
        is Test347State.S02 -> Test347State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test347State): Test347State = when (state) {
        is Test347State.S0 -> Test347State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test347State? = when (stateId) {
        "fail" -> Test347State.Fail
        "pass" -> Test347State.Pass
        "s0" -> Test347State.S0
        "s01" -> Test347State.S01
        "s02" -> Test347State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test347State): String = when (state) {
        is Test347State.Fail -> "fail"
        is Test347State.Pass -> "pass"
        is Test347State.S0 -> "s0"
        is Test347State.S01 -> "s01"
        is Test347State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test347State): Boolean = when (state) {
        is Test347State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test347State): Int = when (state) {
        is Test347State.Fail -> 4
        is Test347State.Pass -> 3
        is Test347State.S0 -> 0
        is Test347State.S01 -> 1
        is Test347State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test347Event? = when (name) {
        "cancel.invoke" -> Test347Event.Cancel.Invoke
        "childToParent" -> Test347Event.ChildToParent
        "done.invoke" -> Test347Event.Done.Invoke
        "error" -> Test347Event.Error.Self
        "error.execution" -> Test347Event.Error.Execution
        "parentToChild" -> Test347Event.ParentToChild
        "timeout" -> Test347Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test347Event): String? = when (event) {
        is Test347Event.Cancel.Invoke -> "cancel.invoke"
        is Test347Event.ChildToParent -> "childToParent"
        is Test347Event.Done.Invoke -> "done.invoke"
        is Test347Event.Error.Self -> "error"
        is Test347Event.Error.Execution -> "error.execution"
        is Test347Event.ParentToChild -> "parentToChild"
        is Test347Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test347State,
        event: Test347Event
    ): TransitionResult<Test347State> = when (state) {
        is Test347State.S0 -> processS0(event)
        is Test347State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test347State.S02 -> {
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
        event: Test347Event
    ): TransitionResult<Test347State> = when {
        event is Test347Event.Timeout -> TransitionResult.External(Test347State.Fail, Test347State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test347Event
    ): TransitionResult<Test347State> = when {
        event is Test347Event.ChildToParent -> TransitionResult.External(Test347State.S02, Test347State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test347Event
    ): TransitionResult<Test347State> = when {
        event is Test347Event.Done.Invoke -> TransitionResult.External(Test347State.Pass, Test347State.S02)

        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test347Event.Error || event is Test347Event.Error.Execution) -> TransitionResult.External(Test347State.Fail, Test347State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test347.scxml:6
    override fun onEntry(state: Test347State) {
        when (state) {
            is Test347State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test347State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test347State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 20000L, Test347Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.child"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test347SceSynthInvokeChildStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("child", childSM, false, Test347Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test347State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test347State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("child", "parentToChild")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test347.scxml:6
    override fun onExit(state: Test347State) {
        when (state) {
            is Test347State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test347State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test347State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("child")
                activeStateIds.remove("s0")
            }
            is Test347State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test347State.S02 -> {
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test347.scxml:6
    override fun executeTransitionActions(
        source: Test347State,
        event: Test347Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
