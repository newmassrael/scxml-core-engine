// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e8782a5c8351481fc8f6e7fcdb09caae80cbe9e47c6019dcf15afff703e3c3b3
// generated-at: 1780407549

// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242.scxml:6

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
    sealed interface Cancel : Test242Event {
        data object Invoke : Cancel
    }
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test242State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test242State): Int = when (state) {
        is Test242State.Fail -> 4
        is Test242State.Pass -> 3
        is Test242State.S0 -> 0
        is Test242State.S02 -> 1
        is Test242State.S03 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test242Event? = when (name) {
        "cancel.invoke" -> Test242Event.Cancel.Invoke
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
        is Test242Event.Cancel.Invoke -> "cancel.invoke"
        is Test242Event.Done.Invoke -> "done.invoke"
        is Test242Event.Error.Execution -> "error.execution"
        is Test242Event.Timeout -> "timeout"
        is Test242Event.Timeout1 -> "timeout1"
        is Test242Event.Timeout2 -> "timeout2"
        is Test242Event.Timeout3 -> "timeout3"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242State,
        event: Test242Event
    ): TransitionResult<Test242State> = when (state) {
        is Test242State.S0 -> processS0(event)
        is Test242State.S02 -> processS02(event)
        is Test242State.S03 -> processS03(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Timeout -> TransitionResult.External(Test242State.Fail, Test242State.S0)

        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.S02, Test242State.S0)

        event is Test242Event.Timeout1 -> TransitionResult.External(Test242State.S03, Test242State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.Pass, Test242State.S02)

        event is Test242Event.Timeout2 -> TransitionResult.External(Test242State.Fail, Test242State.S02)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Timeout3 -> TransitionResult.External(Test242State.Pass, Test242State.S03)

        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.Fail, Test242State.S03)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242.scxml:6
    override fun onEntry(state: Test242State) {
        when (state) {
            is Test242State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test242Event.Timeout1)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242sub1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test242State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return


            scheduleSend("__send_1", 1000L, Test242Event.Timeout2)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242SceSynthInvokeInvoke1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test242State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return


            scheduleSend("__send_2", 1000L, Test242Event.Timeout3)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s03.${System.identityHashCode(this)}._invoke_2"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test242SceSynthInvokeInvoke2StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_2", childSM, false, Test242Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242.scxml:6
    override fun onExit(state: Test242State) {
        when (state) {
            is Test242State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test242State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test242State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test242State.S02 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s02")
            }
            is Test242State.S03 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
                activeStateIds.remove("s03")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test242.scxml:6
    override fun executeTransitionActions(
        source: Test242State,
        event: Test242Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
