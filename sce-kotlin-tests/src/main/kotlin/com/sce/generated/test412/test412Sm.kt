// GENERATED CODE — DO NOT EDIT
// Source: resources/412/test412.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test412

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test412State : State {
    data object Fail : Test412State
    data object Pass : Test412State
    data object S0 : Test412State
    data object S01 : Test412State
    data object S011 : Test412State
    data object S02 : Test412State
    data object S03 : Test412State
    data object S04 : Test412State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test412Event : Event {
    sealed interface Error : Test412Event {
        data object Execution : Error
    }
    data object Event1 : Test412Event
    data object Event2 : Test412Event
    data object Event3 : Test412Event
    data object Timeout : Test412Event
}
// --- State Machine (W3C SCXML) ---

class Test412StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test412State, Test412Event>(scriptEngine) {

    override val initialState: Test412State = Test412State.S011

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test412State): Test412State? = when (state) {
        is Test412State.S01 -> Test412State.S0
        is Test412State.S011 -> Test412State.S01
        is Test412State.S02 -> Test412State.S0
        is Test412State.S03 -> Test412State.S0
        is Test412State.S04 -> Test412State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test412State): Test412State = when (state) {
        is Test412State.S0 -> Test412State.S011
        is Test412State.S01 -> Test412State.S011
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test412State? = when (stateId) {
        "fail" -> Test412State.Fail
        "pass" -> Test412State.Pass
        "s0" -> Test412State.S0
        "s01" -> Test412State.S01
        "s011" -> Test412State.S011
        "s02" -> Test412State.S02
        "s03" -> Test412State.S03
        "s04" -> Test412State.S04
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test412State): String = when (state) {
        is Test412State.Fail -> "fail"
        is Test412State.Pass -> "pass"
        is Test412State.S0 -> "s0"
        is Test412State.S01 -> "s01"
        is Test412State.S011 -> "s011"
        is Test412State.S02 -> "s02"
        is Test412State.S03 -> "s03"
        is Test412State.S04 -> "s04"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test412State): Boolean = when (state) {
        is Test412State.S0 -> false
        is Test412State.S01 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test412State): Int = when (state) {
        is Test412State.Fail -> 7
        is Test412State.Pass -> 6
        is Test412State.S0 -> 0
        is Test412State.S01 -> 1
        is Test412State.S011 -> 2
        is Test412State.S02 -> 3
        is Test412State.S03 -> 4
        is Test412State.S04 -> 5
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test412State,
        event: Test412Event
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test412State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test412State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test412State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S04 -> {
            val result = processS04(event)
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

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test412State
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S011 -> processNullS011()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS011(
    ): TransitionResult<Test412State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test412State.S02, Test412State.S011)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Timeout -> TransitionResult.External(Test412State.Fail, Test412State.S0)

        event is Test412Event.Event1 -> TransitionResult.External(Test412State.Fail, Test412State.S0)

        event is Test412Event.Event2 -> TransitionResult.External(Test412State.Pass, Test412State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event1 -> TransitionResult.External(Test412State.S03, Test412State.S02)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S02)
    }

    private fun processS03(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event2 -> TransitionResult.External(Test412State.S04, Test412State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S03)
    }

    private fun processS04(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event3 -> TransitionResult.External(Test412State.Pass, Test412State.S04)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S04)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test412State) {
        when (state) {
            is Test412State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 1000L, Test412Event.Timeout)
            }
            is Test412State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            raiseInternal(Test412Event.Event1)
                // W3C SCXML 3.3.2: Execute initial transition content
            raiseInternal(Test412Event.Event2)
            }
            is Test412State.S011 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s011")) return
            raiseInternal(Test412Event.Event3)
            }
            is Test412State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test412State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test412State.S04 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test412State) {
        when (state) {
            is Test412State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test412State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test412State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test412State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test412State.S011 -> {
                activeStateIds.remove("s011")
            }
            is Test412State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test412State.S03 -> {
                activeStateIds.remove("s03")
            }
            is Test412State.S04 -> {
                activeStateIds.remove("s04")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test412State,
        event: Test412Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
