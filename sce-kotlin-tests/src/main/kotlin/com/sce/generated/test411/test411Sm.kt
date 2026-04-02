// GENERATED CODE — DO NOT EDIT
// Source: resources/411/test411.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test411

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test411State : State {
    data object Fail : Test411State
    data object Pass : Test411State
    data object S0 : Test411State
    data object S01 : Test411State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test411Event : Event {
    sealed interface Error : Test411Event {
        data object Execution : Error
    }
    data object Event1 : Test411Event
    data object Event2 : Test411Event
    data object Timeout : Test411Event
}
// --- State Machine (W3C SCXML) ---

class Test411StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test411State, Test411Event>(scriptEngine) {

    override val initialState: Test411State = Test411State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        onEntry(Test411State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test411State): Test411State? = when (state) {
        is Test411State.S01 -> Test411State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test411State): Test411State = when (state) {
        is Test411State.S0 -> Test411State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object (for parallel processing)
    override fun resolveState(stateId: String): Test411State? = when (stateId) {
        "fail" -> Test411State.Fail
        "pass" -> Test411State.Pass
        "s0" -> Test411State.S0
        "s01" -> Test411State.S01
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test411State): String = when (state) {
        is Test411State.Fail -> "fail"
        is Test411State.Pass -> "pass"
        is Test411State.S0 -> "s0"
        is Test411State.S01 -> "s01"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test411State): Boolean = when (state) {
        is Test411State.S0 -> false
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test411State): Int = when (state) {
        is Test411State.Fail -> 3
        is Test411State.Pass -> 2
        is Test411State.S0 -> 0
        is Test411State.S01 -> 1
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test411State,
        event: Test411Event
    ): TransitionResult<Test411State> = when (state) {
        is Test411State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test411State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test411Event
    ): TransitionResult<Test411State> = when {
        event is Test411Event.Timeout -> TransitionResult.External(Test411State.Fail)
        event is Test411Event.Event1 -> TransitionResult.External(Test411State.Fail)
        event is Test411Event.Event2 -> TransitionResult.External(Test411State.Pass)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test411State) {
        when (state) {
            is Test411State.Fail -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test411State.Pass -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test411State.S0 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 1000L, Test411Event.Timeout)
            if (isStateActive("s01")) {
            raiseInternal(Test411Event.Event1)
            }
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test411State.S01)
            }
            is Test411State.S01 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s01")) return
            if (isStateActive("s01")) {
            raiseInternal(Test411Event.Event2)
            }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test411State) {
        when (state) {
            is Test411State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test411State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test411State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test411State.S01 -> {
                activeStateIds.remove("s01")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test411State,
        event: Test411Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
