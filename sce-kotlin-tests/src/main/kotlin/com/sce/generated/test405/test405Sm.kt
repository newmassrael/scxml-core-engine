// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712

// GENERATED CODE — DO NOT EDIT
// Source: resources/405/test405.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test405.scxml:6

package com.sce.generated.test405

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test405State : State {
    data object Fail : Test405State
    data object Pass : Test405State
    data object S0 : Test405State
    data object S01p : Test405State
    data object S01p1 : Test405State
    data object S01p11 : Test405State
    data object S01p12 : Test405State
    data object S01p2 : Test405State
    data object S01p21 : Test405State
    data object S01p22 : Test405State
    data object S02 : Test405State
    data object S03 : Test405State
    data object S04 : Test405State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test405Event : Event {
    sealed interface Error : Test405Event {
        data object Execution : Error
    }
    data object Event1 : Test405Event
    data object Event2 : Test405Event
    data object Event3 : Test405Event
    data object Event4 : Test405Event
    data object Timeout : Test405Event
}
// --- State Machine (W3C SCXML) ---

class Test405StateMachine(
) : StateMachineEngine<Test405State, Test405Event>() {

    override val initialState: Test405State = Test405State.S01p11

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test405State): Test405State? = when (state) {
        is Test405State.S01p -> Test405State.S0
        is Test405State.S01p1 -> Test405State.S01p
        is Test405State.S01p11 -> Test405State.S01p1
        is Test405State.S01p12 -> Test405State.S01p1
        is Test405State.S01p2 -> Test405State.S01p
        is Test405State.S01p21 -> Test405State.S01p2
        is Test405State.S01p22 -> Test405State.S01p2
        is Test405State.S02 -> Test405State.S0
        is Test405State.S03 -> Test405State.S0
        is Test405State.S04 -> Test405State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test405State): Test405State = when (state) {
        is Test405State.S0 -> Test405State.S01p11
        is Test405State.S01p -> Test405State.S01p11
        is Test405State.S01p1 -> Test405State.S01p11
        is Test405State.S01p2 -> Test405State.S01p21
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test405State? = when (stateId) {
        "fail" -> Test405State.Fail
        "pass" -> Test405State.Pass
        "s0" -> Test405State.S0
        "s01p" -> Test405State.S01p
        "s01p1" -> Test405State.S01p1
        "s01p11" -> Test405State.S01p11
        "s01p12" -> Test405State.S01p12
        "s01p2" -> Test405State.S01p2
        "s01p21" -> Test405State.S01p21
        "s01p22" -> Test405State.S01p22
        "s02" -> Test405State.S02
        "s03" -> Test405State.S03
        "s04" -> Test405State.S04
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test405State): String = when (state) {
        is Test405State.Fail -> "fail"
        is Test405State.Pass -> "pass"
        is Test405State.S0 -> "s0"
        is Test405State.S01p -> "s01p"
        is Test405State.S01p1 -> "s01p1"
        is Test405State.S01p11 -> "s01p11"
        is Test405State.S01p12 -> "s01p12"
        is Test405State.S01p2 -> "s01p2"
        is Test405State.S01p21 -> "s01p21"
        is Test405State.S01p22 -> "s01p22"
        is Test405State.S02 -> "s02"
        is Test405State.S03 -> "s03"
        is Test405State.S04 -> "s04"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test405State): Boolean = when (state) {
        is Test405State.S0 -> false
        is Test405State.S01p -> false
        is Test405State.S01p1 -> false
        is Test405State.S01p2 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test405State): Boolean = when (state) {
        is Test405State.S01p -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test405State): List<Test405State> = when (state) {
        is Test405State.S01p -> listOf(Test405State.S01p1, Test405State.S01p2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test405State): Int = when (state) {
        is Test405State.Fail -> 12
        is Test405State.Pass -> 11
        is Test405State.S0 -> 0
        is Test405State.S01p -> 4
        is Test405State.S01p1 -> 5
        is Test405State.S01p11 -> 6
        is Test405State.S01p12 -> 7
        is Test405State.S01p2 -> 8
        is Test405State.S01p21 -> 9
        is Test405State.S01p22 -> 10
        is Test405State.S02 -> 1
        is Test405State.S03 -> 2
        is Test405State.S04 -> 3
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test405State,
        event: Test405Event
    ): TransitionResult<Test405State> = when (state) {
        is Test405State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01p1 has no own event transitions)
        is Test405State.S01p1 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p11 has no own event transitions)
        is Test405State.S01p11 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p12 has no own event transitions)
        is Test405State.S01p12 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p2 has no own event transitions)
        is Test405State.S01p2 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p21 has no own event transitions)
        is Test405State.S01p21 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s01p22 has no own event transitions)
        is Test405State.S01p22 -> {
            val anc1 = processS01p(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is Test405State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test405State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test405State.S04 -> {
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
        state: Test405State
    ): TransitionResult<Test405State> = when (state) {
        is Test405State.S01p11 -> processNullS01p11()
        is Test405State.S01p21 -> processNullS01p21()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01p11(
    ): TransitionResult<Test405State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test405State.S01p12, Test405State.S01p11)
    }

    private fun processNullS01p21(
    ): TransitionResult<Test405State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test405State.S01p22, Test405State.S01p21)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test405Event
    ): TransitionResult<Test405State> = when {
        event is Test405Event.Timeout -> TransitionResult.External(Test405State.Fail, Test405State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01p(
        event: Test405Event
    ): TransitionResult<Test405State> = when {
        event is Test405Event.Event1 -> TransitionResult.External(Test405State.S02, Test405State.S01p)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test405Event
    ): TransitionResult<Test405State> = when {
        event is Test405Event.Event2 -> TransitionResult.External(Test405State.S03, Test405State.S02)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test405State.Fail, Test405State.S02)
    }

    private fun processS03(
        event: Test405Event
    ): TransitionResult<Test405State> = when {
        event is Test405Event.Event3 -> TransitionResult.External(Test405State.S04, Test405State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test405State.Fail, Test405State.S03)
    }

    private fun processS04(
        event: Test405Event
    ): TransitionResult<Test405State> = when {
        event is Test405Event.Event4 -> TransitionResult.External(Test405State.Pass, Test405State.S04)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test405State.Fail, Test405State.S04)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test405.scxml:6
    override fun onEntry(state: Test405State) {
        when (state) {
            is Test405State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test405State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test405State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test405Event.Timeout)
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test405State.S01p)
                }
            }
            is Test405State.S01p -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test405State.S01p1)
                onEntry(Test405State.S01p2)
            }
            is Test405State.S01p1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p1")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test405State.S01p11)
                }
            }
            is Test405State.S01p11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p11")) return
            }
            is Test405State.S01p12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p12")) return
            }
            is Test405State.S01p2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p2")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test405State.S01p21)
                }
            }
            is Test405State.S01p21 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p21")) return
            }
            is Test405State.S01p22 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p22")) return
            }
            is Test405State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test405State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test405State.S04 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test405.scxml:6
    override fun onExit(state: Test405State) {
        when (state) {
            is Test405State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test405State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test405State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test405State.S01p -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test405State, Int>>()
                if (activeStateIds.contains("s01p1")) {
                    toExit.add(Test405State.S01p1 to 5)
                }
                if (activeStateIds.contains("s01p11")) {
                    toExit.add(Test405State.S01p11 to 6)
                }
                if (activeStateIds.contains("s01p12")) {
                    toExit.add(Test405State.S01p12 to 7)
                }
                if (activeStateIds.contains("s01p2")) {
                    toExit.add(Test405State.S01p2 to 8)
                }
                if (activeStateIds.contains("s01p21")) {
                    toExit.add(Test405State.S01p21 to 9)
                }
                if (activeStateIds.contains("s01p22")) {
                    toExit.add(Test405State.S01p22 to 10)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s01p")
            }
            is Test405State.S01p1 -> {
                activeStateIds.remove("s01p1")
            }
            is Test405State.S01p11 -> {
                activeStateIds.remove("s01p11")

            raiseInternal(Test405Event.Event2)
            }
            is Test405State.S01p12 -> {
                activeStateIds.remove("s01p12")
            }
            is Test405State.S01p2 -> {
                activeStateIds.remove("s01p2")
            }
            is Test405State.S01p21 -> {
                activeStateIds.remove("s01p21")

            raiseInternal(Test405Event.Event1)
            }
            is Test405State.S01p22 -> {
                activeStateIds.remove("s01p22")
            }
            is Test405State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test405State.S03 -> {
                activeStateIds.remove("s03")
            }
            is Test405State.S04 -> {
                activeStateIds.remove("s04")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test405.scxml:6
    override fun executeTransitionActions(
        source: Test405State,
        event: Test405Event?
    ) {
        when (source) {
        is Test405State.S01p11 -> when {
            event == null -> {

            raiseInternal(Test405Event.Event3)
            }
            else -> {}
        }
        is Test405State.S01p21 -> when {
            event == null -> {

            raiseInternal(Test405Event.Event4)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
