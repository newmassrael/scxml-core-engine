// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b987ea47cf7b98cc29f6a07fbb829bd85b24bd9991a16621d5e7458fb0482788
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/252/test252.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test252.scxml:7 :: _machine

package com.sce.generated.test252

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test252State : State {
    data object Fail : Test252State
    data object Pass : Test252State
    data object S0 : Test252State
    data object S01 : Test252State
    data object S02 : Test252State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test252Event : Event {
    sealed interface Cancel : Test252Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test252Event
    sealed interface Done : Test252Event {
        data object Invoke : Done
    }
    sealed interface Error : Test252Event {
        data object Execution : Error
    }
    data object Foo : Test252Event
    data object Timeout : Test252Event
}
// --- State Machine (W3C SCXML) ---

class Test252StateMachine(
) : StateMachineEngine<Test252State, Test252Event>() {

    override val initialState: Test252State = Test252State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test252State): Test252State? = when (state) {
        is Test252State.S01 -> Test252State.S0
        is Test252State.S02 -> Test252State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test252State): Test252State = when (state) {
        is Test252State.S0 -> Test252State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test252State? = when (stateId) {
        "fail" -> Test252State.Fail
        "pass" -> Test252State.Pass
        "s0" -> Test252State.S0
        "s01" -> Test252State.S01
        "s02" -> Test252State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test252State): String = when (state) {
        is Test252State.Fail -> "fail"
        is Test252State.Pass -> "pass"
        is Test252State.S0 -> "s0"
        is Test252State.S01 -> "s01"
        is Test252State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test252State): Boolean = when (state) {
        is Test252State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test252State): Int = when (state) {
        is Test252State.Fail -> 4
        is Test252State.Pass -> 3
        is Test252State.S0 -> 0
        is Test252State.S01 -> 1
        is Test252State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test252Event? = when (name) {
        "cancel.invoke" -> Test252Event.Cancel.Invoke
        "childToParent" -> Test252Event.ChildToParent
        "done.invoke" -> Test252Event.Done.Invoke
        "error.execution" -> Test252Event.Error.Execution
        "foo" -> Test252Event.Foo
        "timeout" -> Test252Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test252Event): String? = when (event) {
        is Test252Event.Cancel.Invoke -> "cancel.invoke"
        is Test252Event.ChildToParent -> "childToParent"
        is Test252Event.Done.Invoke -> "done.invoke"
        is Test252Event.Error.Execution -> "error.execution"
        is Test252Event.Foo -> "foo"
        is Test252Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test252State,
        event: Test252Event
    ): TransitionResult<Test252State> = when (state) {
        is Test252State.S0 -> processS0(event)
        is Test252State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test252State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test252Event
    ): TransitionResult<Test252State> = when {
        event is Test252Event.Timeout -> TransitionResult.External(Test252State.Pass, Test252State.S0)

        event is Test252Event.ChildToParent -> TransitionResult.External(Test252State.Fail, Test252State.S0)

        event is Test252Event.Done.Invoke -> TransitionResult.External(Test252State.Fail, Test252State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test252Event
    ): TransitionResult<Test252State> = when {
        event is Test252Event.Foo -> TransitionResult.External(Test252State.S02, Test252State.S01)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun onEntry(state: Test252State, pathChild: Test252State?) {
        when (state) {
            is Test252State.Fail -> {
                // SCE-MAP: test252.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test252State.Pass -> {
                // SCE-MAP: test252.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test252State.S0 -> {
                // SCE-MAP: test252.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test252Event.Timeout)
            }
            is Test252State.S01 -> {
                // SCE-MAP: test252.scxml:19 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return


            send(Test252Event.Foo, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test252SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test252Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test252State.S02 -> {
                // SCE-MAP: test252.scxml:45 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun onExit(state: Test252State) {
        when (state) {
            is Test252State.Fail -> {
                // SCE-MAP: test252.scxml:50 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test252State.Pass -> {
                // SCE-MAP: test252.scxml:49 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test252State.S0 -> {
                // SCE-MAP: test252.scxml:10 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test252State.S01 -> {
                // SCE-MAP: test252.scxml:19 :: s01 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s01")
            }
            is Test252State.S02 -> {
                // SCE-MAP: test252.scxml:45 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test252.scxml:7 :: _machine
    override fun executeTransitionActions(
        source: Test252State,
        event: Test252Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
