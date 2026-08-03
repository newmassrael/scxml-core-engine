// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 27e838bbd8708f09c9261661bfb19da674340e525b736fa0c3611ebf1187751e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/413/test413.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test413.scxml:7

package com.sce.generated.test413

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test413State : State {
    data object Fail : Test413State
    data object Pass : Test413State
    data object S1 : Test413State
    data object S2 : Test413State
    data object S2p1 : Test413State
    data object S2p11 : Test413State
    data object S2p111 : Test413State
    data object S2p112 : Test413State
    data object S2p12 : Test413State
    data object S2p121 : Test413State
    data object S2p122 : Test413State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test413Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test413StateMachine(
) : StateMachineEngine<Test413State, Test413Event>() {

    override val initialState: Test413State = Test413State.S2p112

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test413State): Test413State? = when (state) {
        is Test413State.S2p1 -> Test413State.S2
        is Test413State.S2p11 -> Test413State.S2p1
        is Test413State.S2p111 -> Test413State.S2p11
        is Test413State.S2p112 -> Test413State.S2p11
        is Test413State.S2p12 -> Test413State.S2p1
        is Test413State.S2p121 -> Test413State.S2p12
        is Test413State.S2p122 -> Test413State.S2p12
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test413State): Test413State = when (state) {
        is Test413State.S2 -> Test413State.S2p112
        is Test413State.S2p1 -> Test413State.S2p112
        is Test413State.S2p11 -> Test413State.S2p112
        is Test413State.S2p12 -> Test413State.S2p122
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test413State? = when (stateId) {
        "fail" -> Test413State.Fail
        "pass" -> Test413State.Pass
        "s1" -> Test413State.S1
        "s2" -> Test413State.S2
        "s2p1" -> Test413State.S2p1
        "s2p11" -> Test413State.S2p11
        "s2p111" -> Test413State.S2p111
        "s2p112" -> Test413State.S2p112
        "s2p12" -> Test413State.S2p12
        "s2p121" -> Test413State.S2p121
        "s2p122" -> Test413State.S2p122
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test413State): String = when (state) {
        is Test413State.Fail -> "fail"
        is Test413State.Pass -> "pass"
        is Test413State.S1 -> "s1"
        is Test413State.S2 -> "s2"
        is Test413State.S2p1 -> "s2p1"
        is Test413State.S2p11 -> "s2p11"
        is Test413State.S2p111 -> "s2p111"
        is Test413State.S2p112 -> "s2p112"
        is Test413State.S2p12 -> "s2p12"
        is Test413State.S2p121 -> "s2p121"
        is Test413State.S2p122 -> "s2p122"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test413State): Boolean = when (state) {
        is Test413State.S2 -> false
        is Test413State.S2p1 -> false
        is Test413State.S2p11 -> false
        is Test413State.S2p12 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test413State): Boolean = when (state) {
        is Test413State.S2p1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test413State): List<Test413State> = when (state) {
        is Test413State.S2p1 -> listOf(Test413State.S2p11, Test413State.S2p12)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test413State): Int = when (state) {
        is Test413State.Fail -> 10
        is Test413State.Pass -> 9
        is Test413State.S1 -> 0
        is Test413State.S2 -> 1
        is Test413State.S2p1 -> 2
        is Test413State.S2p11 -> 3
        is Test413State.S2p111 -> 4
        is Test413State.S2p112 -> 5
        is Test413State.S2p12 -> 6
        is Test413State.S2p121 -> 7
        is Test413State.S2p122 -> 8
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test413State,
        event: Test413Event
    ): TransitionResult<Test413State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test413State
    ): TransitionResult<Test413State> = when (state) {
        is Test413State.S1 -> processNullS1()
        is Test413State.S2p11 -> processNullS2p1()
        is Test413State.S2p111 -> {
            val null1 = processNullS2p111()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS2p1()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        is Test413State.S2p112 -> {
            val null1 = processNullS2p112()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS2p1()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        is Test413State.S2p12 -> processNullS2p1()
        is Test413State.S2p121 -> {
            val null1 = processNullS2p121()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS2p1()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        is Test413State.S2p122 -> {
            val null1 = processNullS2p122()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS2p1()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test413State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test413State.Fail, Test413State.S1)
    }

    private fun processNullS2p1(
    ): TransitionResult<Test413State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test413State.Fail, Test413State.S2p1)
    }

    private fun processNullS2p111(
    ): TransitionResult<Test413State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test413State.Fail, Test413State.S2p111)
    }

    private fun processNullS2p112(
    ): TransitionResult<Test413State> = when {
        isStateActive("s2p122") -> TransitionResult.External(Test413State.Pass, Test413State.S2p112)
        else -> TransitionResult.Ignored
    }

    private fun processNullS2p121(
    ): TransitionResult<Test413State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test413State.Fail, Test413State.S2p121)
    }

    private fun processNullS2p122(
    ): TransitionResult<Test413State> = when {
        isStateActive("s2p112") -> TransitionResult.External(Test413State.Pass, Test413State.S2p122)
        else -> TransitionResult.Ignored
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test413.scxml:7
    override fun onEntry(state: Test413State) {
        when (state) {
            is Test413State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test413State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test413State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test413State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test413State.S2p1)
                }
            }
            is Test413State.S2p1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p1")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test413State.S2p11)
                onEntry(Test413State.S2p12)
            }
            is Test413State.S2p11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p11")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test413State.S2p112)
                }
            }
            is Test413State.S2p111 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p111")) return
            }
            is Test413State.S2p112 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p112")) return
            }
            is Test413State.S2p12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p12")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test413State.S2p122)
                }
            }
            is Test413State.S2p121 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p121")) return
            }
            is Test413State.S2p122 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2p122")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test413.scxml:7
    override fun onExit(state: Test413State) {
        when (state) {
            is Test413State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test413State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test413State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test413State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test413State.S2p1 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test413State, Int>>()
                if (activeStateIds.contains("s2p11")) {
                    toExit.add(Test413State.S2p11 to 3)
                }
                if (activeStateIds.contains("s2p111")) {
                    toExit.add(Test413State.S2p111 to 4)
                }
                if (activeStateIds.contains("s2p112")) {
                    toExit.add(Test413State.S2p112 to 5)
                }
                if (activeStateIds.contains("s2p12")) {
                    toExit.add(Test413State.S2p12 to 6)
                }
                if (activeStateIds.contains("s2p121")) {
                    toExit.add(Test413State.S2p121 to 7)
                }
                if (activeStateIds.contains("s2p122")) {
                    toExit.add(Test413State.S2p122 to 8)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s2p1")
            }
            is Test413State.S2p11 -> {
                activeStateIds.remove("s2p11")
            }
            is Test413State.S2p111 -> {
                activeStateIds.remove("s2p111")
            }
            is Test413State.S2p112 -> {
                activeStateIds.remove("s2p112")
            }
            is Test413State.S2p12 -> {
                activeStateIds.remove("s2p12")
            }
            is Test413State.S2p121 -> {
                activeStateIds.remove("s2p121")
            }
            is Test413State.S2p122 -> {
                activeStateIds.remove("s2p122")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test413.scxml:7
    override fun executeTransitionActions(
        source: Test413State,
        event: Test413Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
