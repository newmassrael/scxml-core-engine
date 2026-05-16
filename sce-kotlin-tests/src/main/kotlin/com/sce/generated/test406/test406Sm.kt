// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425

// GENERATED CODE — DO NOT EDIT
// Source: resources/406/test406.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test406.scxml:6

package com.sce.generated.test406

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test406State : State {
    data object Fail : Test406State
    data object Pass : Test406State
    data object S0 : Test406State
    data object S01 : Test406State
    data object S01p21 : Test406State
    data object S01p22 : Test406State
    data object S03 : Test406State
    data object S04 : Test406State
    data object S05 : Test406State
    data object S0p2 : Test406State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test406Event : Event {
    sealed interface Error : Test406Event {
        data object Execution : Error
    }
    data object Event1 : Test406Event
    data object Event2 : Test406Event
    data object Event3 : Test406Event
    data object Event4 : Test406Event
    data object Timeout : Test406Event
}
// --- State Machine (W3C SCXML) ---

class Test406StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test406State, Test406Event>(scriptEngine) {

    override val initialState: Test406State = Test406State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test406State): Test406State? = when (state) {
        is Test406State.S01 -> Test406State.S0
        is Test406State.S01p21 -> Test406State.S0p2
        is Test406State.S01p22 -> Test406State.S0p2
        is Test406State.S03 -> Test406State.S0
        is Test406State.S04 -> Test406State.S0
        is Test406State.S05 -> Test406State.S0
        is Test406State.S0p2 -> Test406State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test406State): Test406State = when (state) {
        is Test406State.S0 -> Test406State.S01
        is Test406State.S0p2 -> Test406State.S01p21
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test406State? = when (stateId) {
        "fail" -> Test406State.Fail
        "pass" -> Test406State.Pass
        "s0" -> Test406State.S0
        "s01" -> Test406State.S01
        "s01p21" -> Test406State.S01p21
        "s01p22" -> Test406State.S01p22
        "s03" -> Test406State.S03
        "s04" -> Test406State.S04
        "s05" -> Test406State.S05
        "s0p2" -> Test406State.S0p2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test406State): String = when (state) {
        is Test406State.Fail -> "fail"
        is Test406State.Pass -> "pass"
        is Test406State.S0 -> "s0"
        is Test406State.S01 -> "s01"
        is Test406State.S01p21 -> "s01p21"
        is Test406State.S01p22 -> "s01p22"
        is Test406State.S03 -> "s03"
        is Test406State.S04 -> "s04"
        is Test406State.S05 -> "s05"
        is Test406State.S0p2 -> "s0p2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test406State): Boolean = when (state) {
        is Test406State.S0 -> false
        is Test406State.S0p2 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test406State): Boolean = when (state) {
        is Test406State.S0p2 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test406State): List<Test406State> = when (state) {
        is Test406State.S0p2 -> listOf(Test406State.S01p21, Test406State.S01p22)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test406State): Int = when (state) {
        is Test406State.Fail -> 9
        is Test406State.Pass -> 8
        is Test406State.S0 -> 0
        is Test406State.S01 -> 1
        is Test406State.S01p21 -> 6
        is Test406State.S01p22 -> 7
        is Test406State.S03 -> 2
        is Test406State.S04 -> 3
        is Test406State.S05 -> 4
        is Test406State.S0p2 -> 5
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test406State,
        event: Test406Event
    ): TransitionResult<Test406State> = when (state) {
        is Test406State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test406State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p21 has no own event transitions)
        is Test406State.S01p21 -> {
            val anc1 = processS0p2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p22 has no own event transitions)
        is Test406State.S01p22 -> {
            val anc1 = processS0p2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is Test406State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test406State.S04 -> {
            val result = processS04(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test406State.S05 -> {
            val result = processS05(event)
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
        state: Test406State
    ): TransitionResult<Test406State> = when (state) {
        is Test406State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test406State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test406State.S0p2, Test406State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test406Event
    ): TransitionResult<Test406State> = when {
        event is Test406Event.Timeout -> TransitionResult.External(Test406State.Fail, Test406State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test406Event
    ): TransitionResult<Test406State> = when {
        event is Test406Event.Event2 -> TransitionResult.External(Test406State.S04, Test406State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test406State.Fail, Test406State.S03)
    }

    private fun processS04(
        event: Test406Event
    ): TransitionResult<Test406State> = when {
        event is Test406Event.Event3 -> TransitionResult.External(Test406State.S05, Test406State.S04)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test406State.Fail, Test406State.S04)
    }

    private fun processS05(
        event: Test406Event
    ): TransitionResult<Test406State> = when {
        event is Test406Event.Event4 -> TransitionResult.External(Test406State.Pass, Test406State.S05)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test406State.Fail, Test406State.S05)
    }

    private fun processS0p2(
        event: Test406Event
    ): TransitionResult<Test406State> = when {
        event is Test406Event.Event1 -> TransitionResult.External(Test406State.S03, Test406State.S0p2)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test406.scxml:6
    override fun onEntry(state: Test406State) {
        when (state) {
            is Test406State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test406State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test406State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test406Event.Timeout)
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test406State.S01)
                }
            }
            is Test406State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test406State.S01p21 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p21")) return

            raiseInternal(Test406Event.Event3)
            }
            is Test406State.S01p22 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p22")) return

            raiseInternal(Test406Event.Event4)
            }
            is Test406State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test406State.S04 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return
            }
            is Test406State.S05 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s05")) return
            }
            is Test406State.S0p2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0p2")) return

            raiseInternal(Test406Event.Event2)
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test406State.S01p21)
                onEntry(Test406State.S01p22)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test406.scxml:6
    override fun onExit(state: Test406State) {
        when (state) {
            is Test406State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test406State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test406State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test406State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test406State.S01p21 -> {
                activeStateIds.remove("s01p21")
            }
            is Test406State.S01p22 -> {
                activeStateIds.remove("s01p22")
            }
            is Test406State.S03 -> {
                activeStateIds.remove("s03")
            }
            is Test406State.S04 -> {
                activeStateIds.remove("s04")
            }
            is Test406State.S05 -> {
                activeStateIds.remove("s05")
            }
            is Test406State.S0p2 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test406State, Int>>()
                if (activeStateIds.contains("s01p21")) {
                    toExit.add(Test406State.S01p21 to 6)
                }
                if (activeStateIds.contains("s01p22")) {
                    toExit.add(Test406State.S01p22 to 7)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s0p2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test406.scxml:6
    override fun executeTransitionActions(
        source: Test406State,
        event: Test406Event?
    ) {
        when (source) {
        is Test406State.S01 -> when {
            event == null -> {

            raiseInternal(Test406Event.Event1)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
