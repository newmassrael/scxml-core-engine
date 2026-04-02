// GENERATED CODE — DO NOT EDIT
// Source: resources/399/test399.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test399

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test399State : State {
    data object Fail : Test399State
    data object Pass : Test399State
    data object S0 : Test399State
    data object S01 : Test399State
    data object S02 : Test399State
    data object S03 : Test399State
    data object S04 : Test399State
    data object S05 : Test399State
    data object S06 : Test399State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test399Event : Event {
    data object Bar : Test399Event
    sealed interface Error : Test399Event {
        data object Execution : Error
    }
    sealed interface Foo : Test399Event {
        data object Self : Foo
        data object Zoo : Foo
    }
    data object Foos : Test399Event
    data object Timeout : Test399Event
}
// --- State Machine (W3C SCXML) ---

class Test399StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test399State, Test399Event>(scriptEngine) {

    override val initialState: Test399State = Test399State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test399State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test399State): Test399State? = when (state) {
        is Test399State.S01 -> Test399State.S0
        is Test399State.S02 -> Test399State.S0
        is Test399State.S03 -> Test399State.S0
        is Test399State.S04 -> Test399State.S0
        is Test399State.S05 -> Test399State.S0
        is Test399State.S06 -> Test399State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test399State): Test399State = when (state) {
        is Test399State.S0 -> Test399State.S01
        else -> state
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test399State,
        event: Test399Event
    ): TransitionResult<Test399State> = when (state) {
        is Test399State.S0 -> processS0(event)
        is Test399State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S04 -> {
            val result = processS04(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S05 -> {
            val result = processS05(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test399State.S06 -> {
            val result = processS06(event)
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
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        event is Test399Event.Timeout -> TransitionResult.External(Test399State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S02)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S03)
        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo bar"
        (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.S04)
        else -> TransitionResult.Ignored
    }

    private fun processS04(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Prefix match for "foo"
        (event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> TransitionResult.External(Test399State.Fail)
        event is Test399Event.Foos -> TransitionResult.External(Test399State.S05)
        else -> TransitionResult.Ignored
    }

    private fun processS05(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        event is Test399Event.Foo.Zoo -> TransitionResult.External(Test399State.S06)
        else -> TransitionResult.Ignored
    }

    private fun processS06(
        event: Test399Event
    ): TransitionResult<Test399State> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test399State.Pass)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test399State) {
        when (state) {
            is Test399State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.S0 -> {
            scheduleSend("__send_0", 2000L, Test399Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test399State.S01)
            }
            is Test399State.S01 -> {
            raiseInternal(Test399Event.Foo.Self)
            }
            is Test399State.S02 -> {
            raiseInternal(Test399Event.Bar)
            }
            is Test399State.S03 -> {
            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S04 -> {
            raiseInternal(Test399Event.Foos)
            }
            is Test399State.S05 -> {
            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S06 -> {
            raiseInternal(Test399Event.Foo.Self)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test399State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test399State,
        event: Test399Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
