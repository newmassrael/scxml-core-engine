// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1cfb591080ee0f7028d74f99302d8ee6d7a5b2416447e2ddc2e71e093c1a3c98
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/404/test404.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test404.scxml:7

package com.sce.generated.test404

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test404State : State {
    data object Fail : Test404State
    data object Pass : Test404State
    data object S0 : Test404State
    data object S01p : Test404State
    data object S01p1 : Test404State
    data object S01p2 : Test404State
    data object S02 : Test404State
    data object S03 : Test404State
    data object S04 : Test404State
    data object S05 : Test404State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test404Event : Event {
    data object Event1 : Test404Event
    data object Event2 : Test404Event
    data object Event3 : Test404Event
    data object Event4 : Test404Event
}
// --- State Machine (W3C SCXML) ---

class Test404StateMachine(
) : StateMachineEngine<Test404State, Test404Event>() {

    override val initialState: Test404State = Test404State.S01p1

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test404State): Test404State? = when (state) {
        is Test404State.S01p -> Test404State.S0
        is Test404State.S01p1 -> Test404State.S01p
        is Test404State.S01p2 -> Test404State.S01p
        is Test404State.S02 -> Test404State.S0
        is Test404State.S03 -> Test404State.S0
        is Test404State.S04 -> Test404State.S0
        is Test404State.S05 -> Test404State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test404State): Test404State = when (state) {
        is Test404State.S0 -> Test404State.S01p1
        is Test404State.S01p -> Test404State.S01p1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test404State? = when (stateId) {
        "fail" -> Test404State.Fail
        "pass" -> Test404State.Pass
        "s0" -> Test404State.S0
        "s01p" -> Test404State.S01p
        "s01p1" -> Test404State.S01p1
        "s01p2" -> Test404State.S01p2
        "s02" -> Test404State.S02
        "s03" -> Test404State.S03
        "s04" -> Test404State.S04
        "s05" -> Test404State.S05
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test404State): String = when (state) {
        is Test404State.Fail -> "fail"
        is Test404State.Pass -> "pass"
        is Test404State.S0 -> "s0"
        is Test404State.S01p -> "s01p"
        is Test404State.S01p1 -> "s01p1"
        is Test404State.S01p2 -> "s01p2"
        is Test404State.S02 -> "s02"
        is Test404State.S03 -> "s03"
        is Test404State.S04 -> "s04"
        is Test404State.S05 -> "s05"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test404State): Boolean = when (state) {
        is Test404State.S0 -> false
        is Test404State.S01p -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test404State): Boolean = when (state) {
        is Test404State.S01p -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test404State): List<Test404State> = when (state) {
        is Test404State.S01p -> listOf(Test404State.S01p1, Test404State.S01p2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test404State): Int = when (state) {
        is Test404State.Fail -> 9
        is Test404State.Pass -> 8
        is Test404State.S0 -> 0
        is Test404State.S01p -> 1
        is Test404State.S01p1 -> 2
        is Test404State.S01p2 -> 3
        is Test404State.S02 -> 4
        is Test404State.S03 -> 5
        is Test404State.S04 -> 6
        is Test404State.S05 -> 7
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test404State,
        event: Test404Event
    ): TransitionResult<Test404State> = when (state) {
        is Test404State.S02 -> processS02(event)
        is Test404State.S03 -> processS03(event)
        is Test404State.S04 -> processS04(event)
        is Test404State.S05 -> processS05(event)
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test404State
    ): TransitionResult<Test404State> = when (state) {
        is Test404State.S01p1 -> processNullS01p()
        is Test404State.S01p2 -> processNullS01p()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01p(
    ): TransitionResult<Test404State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test404State.S02, Test404State.S01p)
    }

    // --- Per-State Event Handlers ---

    private fun processS02(
        event: Test404Event
    ): TransitionResult<Test404State> = when {
        event is Test404Event.Event1 -> TransitionResult.External(Test404State.S03, Test404State.S02)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test404State.Fail, Test404State.S02)
    }

    private fun processS03(
        event: Test404Event
    ): TransitionResult<Test404State> = when {
        event is Test404Event.Event2 -> TransitionResult.External(Test404State.S04, Test404State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test404State.Fail, Test404State.S03)
    }

    private fun processS04(
        event: Test404Event
    ): TransitionResult<Test404State> = when {
        event is Test404Event.Event3 -> TransitionResult.External(Test404State.S05, Test404State.S04)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test404State.Fail, Test404State.S04)
    }

    private fun processS05(
        event: Test404Event
    ): TransitionResult<Test404State> = when {
        event is Test404Event.Event4 -> TransitionResult.External(Test404State.Pass, Test404State.S05)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test404State.Fail, Test404State.S05)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test404.scxml:7
    override fun onEntry(state: Test404State) {
        when (state) {
            is Test404State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test404State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test404State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test404State.S01p)
                }
            }
            is Test404State.S01p -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test404State.S01p1)
                onEntry(Test404State.S01p2)
            }
            is Test404State.S01p1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p1")) return
            }
            is Test404State.S01p2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p2")) return
            }
            is Test404State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test404State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test404State.S04 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return
            }
            is Test404State.S05 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s05")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test404.scxml:7
    override fun onExit(state: Test404State) {
        when (state) {
            is Test404State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test404State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test404State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test404State.S01p -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test404State, Int>>()
                if (activeStateIds.contains("s01p1")) {
                    toExit.add(Test404State.S01p1 to 2)
                }
                if (activeStateIds.contains("s01p2")) {
                    toExit.add(Test404State.S01p2 to 3)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s01p")

            raiseInternal(Test404Event.Event3)
            }
            is Test404State.S01p1 -> {
                activeStateIds.remove("s01p1")

            raiseInternal(Test404Event.Event2)
            }
            is Test404State.S01p2 -> {
                activeStateIds.remove("s01p2")

            raiseInternal(Test404Event.Event1)
            }
            is Test404State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test404State.S03 -> {
                activeStateIds.remove("s03")
            }
            is Test404State.S04 -> {
                activeStateIds.remove("s04")
            }
            is Test404State.S05 -> {
                activeStateIds.remove("s05")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test404.scxml:7
    override fun executeTransitionActions(
        source: Test404State,
        event: Test404Event?
    ) {
        when (source) {
        is Test404State.S01p -> when {
            event == null -> {

            raiseInternal(Test404Event.Event4)
            }
            else -> {}
        }
        is Test404State.S01p1 -> when {
            event == null -> {

            raiseInternal(Test404Event.Event4)
            }
            else -> {}
        }
        is Test404State.S01p2 -> when {
            event == null -> {

            raiseInternal(Test404Event.Event4)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
