// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912

// GENERATED CODE — DO NOT EDIT
// Source: resources/235/test235.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test235.scxml:6

package com.sce.generated.test235

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test235State : State {
    data object Fail : Test235State
    data object Pass : Test235State
    data object S0 : Test235State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test235Event : Event {
    sealed interface Cancel : Test235Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test235Event {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Foo : Invoke
        }
    }
    sealed interface Error : Test235Event {
        data object Execution : Error
    }
    data object Timeout : Test235Event
}
// --- State Machine (W3C SCXML) ---

class Test235StateMachine(
) : StateMachineEngine<Test235State, Test235Event>() {

    override val initialState: Test235State = Test235State.S0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test235State? = when (stateId) {
        "fail" -> Test235State.Fail
        "pass" -> Test235State.Pass
        "s0" -> Test235State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test235State): String = when (state) {
        is Test235State.Fail -> "fail"
        is Test235State.Pass -> "pass"
        is Test235State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test235State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test235State): Int = when (state) {
        is Test235State.Fail -> 2
        is Test235State.Pass -> 1
        is Test235State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test235Event? = when (name) {
        "cancel.invoke" -> Test235Event.Cancel.Invoke
        "done.invoke" -> Test235Event.Done.Invoke.Self
        "done.invoke.foo" -> Test235Event.Done.Invoke.Foo
        "error.execution" -> Test235Event.Error.Execution
        "timeout" -> Test235Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test235Event): String? = when (event) {
        is Test235Event.Cancel.Invoke -> "cancel.invoke"
        is Test235Event.Done.Invoke.Self -> "done.invoke"
        is Test235Event.Done.Invoke.Foo -> "done.invoke.foo"
        is Test235Event.Error.Execution -> "error.execution"
        is Test235Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test235State,
        event: Test235Event
    ): TransitionResult<Test235State> = when (state) {
        is Test235State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test235Event
    ): TransitionResult<Test235State> = when {
        event is Test235Event.Done.Invoke.Foo -> TransitionResult.External(Test235State.Pass, Test235State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test235State.Fail, Test235State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test235.scxml:6
    override fun onEntry(state: Test235State) {
        when (state) {
            is Test235State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test235State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test235Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.foo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test235SceSynthInvokeFooStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("foo", childSM, false, Test235Event.Done.Invoke.Foo, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test235.scxml:6
    override fun onExit(state: Test235State) {
        when (state) {
            is Test235State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test235State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test235State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("foo")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test235.scxml:6
    override fun executeTransitionActions(
        source: Test235State,
        event: Test235Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
