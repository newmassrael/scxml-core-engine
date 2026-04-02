// GENERATED CODE — DO NOT EDIT
// Source: resources/387/test387.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test387

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test387State : State {
    data object Fail : Test387State
    data object Pass : Test387State
    data object S0 : Test387State
    data object S01 : Test387State
    data object S011 : Test387State
    data object S012 : Test387State
    data object S02 : Test387State
    data object S021 : Test387State
    data object S022 : Test387State
    data object S1 : Test387State
    data object S11 : Test387State
    data object S111 : Test387State
    data object S112 : Test387State
    data object S12 : Test387State
    data object S121 : Test387State
    data object S122 : Test387State
    data object S3 : Test387State
    data object S4 : Test387State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test387Event : Event {
    data object EnteringS011 : Test387Event
    data object EnteringS012 : Test387Event
    data object EnteringS021 : Test387Event
    data object EnteringS022 : Test387Event
    data object EnteringS111 : Test387Event
    data object EnteringS112 : Test387Event
    data object EnteringS121 : Test387Event
    data object EnteringS122 : Test387Event
    sealed interface Error : Test387Event {
        data object Execution : Error
    }
    data object Timeout : Test387Event
}
// --- State Machine (W3C SCXML) ---

class Test387StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test387State, Test387Event>(scriptEngine) {

    override val initialState: Test387State = Test387State.S3

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test387State): Test387State? = when (state) {
        is Test387State.S01 -> Test387State.S0
        is Test387State.S011 -> Test387State.S01
        is Test387State.S012 -> Test387State.S01
        is Test387State.S02 -> Test387State.S0
        is Test387State.S021 -> Test387State.S02
        is Test387State.S022 -> Test387State.S02
        is Test387State.S11 -> Test387State.S1
        is Test387State.S111 -> Test387State.S11
        is Test387State.S112 -> Test387State.S11
        is Test387State.S12 -> Test387State.S1
        is Test387State.S121 -> Test387State.S12
        is Test387State.S122 -> Test387State.S12
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test387State): Test387State = when (state) {
        is Test387State.S0 -> Test387State.S011
        is Test387State.S01 -> Test387State.S011
        is Test387State.S02 -> Test387State.S021
        is Test387State.S1 -> Test387State.S111
        is Test387State.S11 -> Test387State.S111
        is Test387State.S12 -> Test387State.S121
        else -> state
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test387State,
        event: Test387Event
    ): TransitionResult<Test387State> = when (state) {
        is Test387State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test387State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test387State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s012 has no own event transitions)
        is Test387State.S012 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test387State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s021 has no own event transitions)
        is Test387State.S021 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s022 has no own event transitions)
        is Test387State.S022 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test387State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test387State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test387State.S111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s112 has no own event transitions)
        is Test387State.S112 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s12 has no own event transitions)
        is Test387State.S12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s121 has no own event transitions)
        is Test387State.S121 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s122 has no own event transitions)
        is Test387State.S122 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test387State
    ): TransitionResult<Test387State> = when (state) {
        is Test387State.S3 -> processNullS3()
        is Test387State.S4 -> processNullS4()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS3(
    ): TransitionResult<Test387State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test387State.S011)
    }

    private fun processNullS4(
    ): TransitionResult<Test387State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test387State.S122)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test387Event
    ): TransitionResult<Test387State> = when {
        event is Test387Event.EnteringS011 -> TransitionResult.External(Test387State.S4, Test387State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test387State.Fail)
    }

    private fun processS1(
        event: Test387Event
    ): TransitionResult<Test387State> = when {
        event is Test387Event.EnteringS122 -> TransitionResult.External(Test387State.Pass, Test387State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test387State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test387State) {
        when (state) {
            is Test387State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.S0 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S01)
            }
            is Test387State.S01 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S011)
            }
            is Test387State.S011 -> {
            raiseInternal(Test387Event.EnteringS011)
            }
            is Test387State.S012 -> {
            raiseInternal(Test387Event.EnteringS012)
            }
            is Test387State.S02 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S021)
            }
            is Test387State.S021 -> {
            raiseInternal(Test387Event.EnteringS021)
            }
            is Test387State.S022 -> {
            raiseInternal(Test387Event.EnteringS022)
            }
            is Test387State.S1 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S11)
            }
            is Test387State.S11 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S111)
            }
            is Test387State.S111 -> {
            raiseInternal(Test387Event.EnteringS111)
            }
            is Test387State.S112 -> {
            raiseInternal(Test387Event.EnteringS112)
            }
            is Test387State.S12 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test387State.S121)
            }
            is Test387State.S121 -> {
            raiseInternal(Test387Event.EnteringS121)
            }
            is Test387State.S122 -> {
            raiseInternal(Test387Event.EnteringS122)
            }
            is Test387State.S3 -> {
            scheduleSend("__send_0", 1000L, Test387Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test387State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test387State,
        event: Test387Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
