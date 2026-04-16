
// GENERATED CODE — DO NOT EDIT
// Source: resources/421/test421.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test421

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test421State : State {
    data object Fail : Test421State
    data object Pass : Test421State
    data object S1 : Test421State
    data object S11 : Test421State
    data object S12 : Test421State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test421Event : Event {
    sealed interface Error : Test421Event {
        data object Execution : Error
    }
    data object ExternalEvent : Test421Event
    data object InternalEvent1 : Test421Event
    data object InternalEvent2 : Test421Event
    data object InternalEvent3 : Test421Event
    data object InternalEvent4 : Test421Event
}
// --- State Machine (W3C SCXML) ---

class Test421StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test421State, Test421Event>(scriptEngine) {

    override val initialState: Test421State = Test421State.S11

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test421State): Test421State? = when (state) {
        is Test421State.S11 -> Test421State.S1
        is Test421State.S12 -> Test421State.S1
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test421State): Test421State = when (state) {
        is Test421State.S1 -> Test421State.S11
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test421State? = when (stateId) {
        "fail" -> Test421State.Fail
        "pass" -> Test421State.Pass
        "s1" -> Test421State.S1
        "s11" -> Test421State.S11
        "s12" -> Test421State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test421State): String = when (state) {
        is Test421State.Fail -> "fail"
        is Test421State.Pass -> "pass"
        is Test421State.S1 -> "s1"
        is Test421State.S11 -> "s11"
        is Test421State.S12 -> "s12"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test421State): Boolean = when (state) {
        is Test421State.S1 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test421State): Int = when (state) {
        is Test421State.Fail -> 4
        is Test421State.Pass -> 3
        is Test421State.S1 -> 0
        is Test421State.S11 -> 1
        is Test421State.S12 -> 2
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test421State,
        event: Test421Event
    ): TransitionResult<Test421State> = when (state) {
        is Test421State.S1 -> processS1(event)
        is Test421State.S11 -> {
            val result = processS11(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test421State.S12 -> {
            val result = processS12(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test421Event
    ): TransitionResult<Test421State> = when {
        event is Test421Event.ExternalEvent -> TransitionResult.External(Test421State.Fail, Test421State.S1)

        else -> TransitionResult.Ignored
    }

    private fun processS11(
        event: Test421Event
    ): TransitionResult<Test421State> = when {
        event is Test421Event.InternalEvent3 -> TransitionResult.External(Test421State.S12, Test421State.S11)

        else -> TransitionResult.Ignored
    }

    private fun processS12(
        event: Test421Event
    ): TransitionResult<Test421State> = when {
        event is Test421Event.InternalEvent4 -> TransitionResult.External(Test421State.Pass, Test421State.S12)

        else -> TransitionResult.Ignored
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test421State) {
        when (state) {
            is Test421State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test421State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test421State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            send(Test421Event.ExternalEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))

            raiseInternal(Test421Event.InternalEvent1)

            raiseInternal(Test421Event.InternalEvent2)

            raiseInternal(Test421Event.InternalEvent3)

            raiseInternal(Test421Event.InternalEvent4)
            }
            is Test421State.S11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
            }
            is Test421State.S12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s12")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test421State) {
        when (state) {
            is Test421State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test421State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test421State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test421State.S11 -> {
                activeStateIds.remove("s11")
            }
            is Test421State.S12 -> {
                activeStateIds.remove("s12")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test421State,
        event: Test421Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
