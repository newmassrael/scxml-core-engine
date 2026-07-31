// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/417/test417.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test417.scxml:7

package com.sce.generated.test417

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test417State : State {
    data object Fail : Test417State
    data object Pass : Test417State
    data object S1 : Test417State
    data object S1p1 : Test417State
    data object S1p11 : Test417State
    data object S1p111 : Test417State
    data object S1p11final : Test417State
    data object S1p12 : Test417State
    data object S1p121 : Test417State
    data object S1p12final : Test417State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test417Event : Event {
    sealed interface Done : Test417Event {
        sealed interface State : Done {
            data object S1p1 : State
            data object S1p11 : State
            data object S1p12 : State
        }
    }
    sealed interface Error : Test417Event {
        data object Execution : Error
    }
    data object Timeout : Test417Event
}
// --- State Machine (W3C SCXML) ---

class Test417StateMachine(
) : StateMachineEngine<Test417State, Test417Event>() {

    override val initialState: Test417State = Test417State.S1p111

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test417State): Test417State? = when (state) {
        is Test417State.S1p1 -> Test417State.S1
        is Test417State.S1p11 -> Test417State.S1p1
        is Test417State.S1p111 -> Test417State.S1p11
        is Test417State.S1p11final -> Test417State.S1p11
        is Test417State.S1p12 -> Test417State.S1p1
        is Test417State.S1p121 -> Test417State.S1p12
        is Test417State.S1p12final -> Test417State.S1p12
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test417State): Test417State = when (state) {
        is Test417State.S1 -> Test417State.S1p111
        is Test417State.S1p1 -> Test417State.S1p111
        is Test417State.S1p11 -> Test417State.S1p111
        is Test417State.S1p12 -> Test417State.S1p121
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test417State? = when (stateId) {
        "fail" -> Test417State.Fail
        "pass" -> Test417State.Pass
        "s1" -> Test417State.S1
        "s1p1" -> Test417State.S1p1
        "s1p11" -> Test417State.S1p11
        "s1p111" -> Test417State.S1p111
        "s1p11final" -> Test417State.S1p11final
        "s1p12" -> Test417State.S1p12
        "s1p121" -> Test417State.S1p121
        "s1p12final" -> Test417State.S1p12final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test417State): String = when (state) {
        is Test417State.Fail -> "fail"
        is Test417State.Pass -> "pass"
        is Test417State.S1 -> "s1"
        is Test417State.S1p1 -> "s1p1"
        is Test417State.S1p11 -> "s1p11"
        is Test417State.S1p111 -> "s1p111"
        is Test417State.S1p11final -> "s1p11final"
        is Test417State.S1p12 -> "s1p12"
        is Test417State.S1p121 -> "s1p121"
        is Test417State.S1p12final -> "s1p12final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test417State): Boolean = when (state) {
        is Test417State.S1 -> false
        is Test417State.S1p1 -> false
        is Test417State.S1p11 -> false
        is Test417State.S1p12 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test417State): Boolean = when (state) {
        is Test417State.S1p1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test417State): List<Test417State> = when (state) {
        is Test417State.S1p1 -> listOf(Test417State.S1p11, Test417State.S1p12)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test417State): Int = when (state) {
        is Test417State.Fail -> 9
        is Test417State.Pass -> 8
        is Test417State.S1 -> 0
        is Test417State.S1p1 -> 1
        is Test417State.S1p11 -> 2
        is Test417State.S1p111 -> 3
        is Test417State.S1p11final -> 4
        is Test417State.S1p12 -> 5
        is Test417State.S1p121 -> 6
        is Test417State.S1p12final -> 7
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test417State,
        event: Test417Event
    ): TransitionResult<Test417State> = when (state) {
        is Test417State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s1p11 has no own event transitions)
        is Test417State.S1p11 -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s1p111 has no own event transitions)
        is Test417State.S1p111 -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s1p11final has no own event transitions)
        is Test417State.S1p11final -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s1p12 has no own event transitions)
        is Test417State.S1p12 -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s1p121 has no own event transitions)
        is Test417State.S1p121 -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s1p12final has no own event transitions)
        is Test417State.S1p12final -> {
            val anc1 = processS1p1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test417State
    ): TransitionResult<Test417State> = when (state) {
        is Test417State.S1p111 -> processNullS1p111()
        is Test417State.S1p121 -> processNullS1p121()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1p111(
    ): TransitionResult<Test417State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test417State.S1p11final, Test417State.S1p111)
    }

    private fun processNullS1p121(
    ): TransitionResult<Test417State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test417State.S1p12final, Test417State.S1p121)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test417Event
    ): TransitionResult<Test417State> = when {
        event is Test417Event.Timeout -> TransitionResult.External(Test417State.Fail, Test417State.S1)

        else -> TransitionResult.Ignored
    }

    private fun processS1p1(
        event: Test417Event
    ): TransitionResult<Test417State> = when {
        event is Test417Event.Done.State.S1p1 -> TransitionResult.External(Test417State.Pass, Test417State.S1p1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test417.scxml:7
    override fun onEntry(state: Test417State) {
        when (state) {
            is Test417State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test417State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test417State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_0", 1000L, Test417Event.Timeout)
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test417State.S1p1)
                }
            }
            is Test417State.S1p1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p1")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test417State.S1p11)
                onEntry(Test417State.S1p12)
            }
            is Test417State.S1p11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p11")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test417State.S1p111)
                }
            }
            is Test417State.S1p111 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p111")) return
            }
            is Test417State.S1p11final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p11final")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test417Event.Done.State.S1p11, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("s1p11final")) && (activeStateIds.contains("s1p12final"))) {
                    raiseInternal(Test417Event.Done.State.S1p1)
                }
            }
            is Test417State.S1p12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p12")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test417State.S1p121)
                }
            }
            is Test417State.S1p121 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p121")) return
            }
            is Test417State.S1p12final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1p12final")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test417Event.Done.State.S1p12, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("s1p11final")) && (activeStateIds.contains("s1p12final"))) {
                    raiseInternal(Test417Event.Done.State.S1p1)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test417.scxml:7
    override fun onExit(state: Test417State) {
        when (state) {
            is Test417State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test417State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test417State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test417State.S1p1 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test417State, Int>>()
                if (activeStateIds.contains("s1p11")) {
                    toExit.add(Test417State.S1p11 to 2)
                }
                if (activeStateIds.contains("s1p111")) {
                    toExit.add(Test417State.S1p111 to 3)
                }
                if (activeStateIds.contains("s1p11final")) {
                    toExit.add(Test417State.S1p11final to 4)
                }
                if (activeStateIds.contains("s1p12")) {
                    toExit.add(Test417State.S1p12 to 5)
                }
                if (activeStateIds.contains("s1p121")) {
                    toExit.add(Test417State.S1p121 to 6)
                }
                if (activeStateIds.contains("s1p12final")) {
                    toExit.add(Test417State.S1p12final to 7)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s1p1")
            }
            is Test417State.S1p11 -> {
                activeStateIds.remove("s1p11")
            }
            is Test417State.S1p111 -> {
                activeStateIds.remove("s1p111")
            }
            is Test417State.S1p11final -> {
                activeStateIds.remove("s1p11final")
            }
            is Test417State.S1p12 -> {
                activeStateIds.remove("s1p12")
            }
            is Test417State.S1p121 -> {
                activeStateIds.remove("s1p121")
            }
            is Test417State.S1p12final -> {
                activeStateIds.remove("s1p12final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test417.scxml:7
    override fun executeTransitionActions(
        source: Test417State,
        event: Test417Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
