// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781102364

// GENERATED CODE — DO NOT EDIT
// Source: resources/451/test451.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test451.scxml:5

package com.sce.generated.test451

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test451State : State {
    data object Fail : Test451State
    data object P : Test451State
    data object Pass : Test451State
    data object S0 : Test451State
    data object S1 : Test451State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test451Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test451StateMachine(
) : StateMachineEngine<Test451State, Test451Event>() {

    override val initialState: Test451State = Test451State.S0

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test451State): Test451State? = when (state) {
        is Test451State.S0 -> Test451State.P
        is Test451State.S1 -> Test451State.P
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test451State): Test451State = when (state) {
        is Test451State.P -> Test451State.S0
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test451State? = when (stateId) {
        "fail" -> Test451State.Fail
        "p" -> Test451State.P
        "pass" -> Test451State.Pass
        "s0" -> Test451State.S0
        "s1" -> Test451State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test451State): String = when (state) {
        is Test451State.Fail -> "fail"
        is Test451State.P -> "p"
        is Test451State.Pass -> "pass"
        is Test451State.S0 -> "s0"
        is Test451State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test451State): Boolean = when (state) {
        is Test451State.P -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test451State): Boolean = when (state) {
        is Test451State.P -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test451State): List<Test451State> = when (state) {
        is Test451State.P -> listOf(Test451State.S0, Test451State.S1)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test451State): Int = when (state) {
        is Test451State.Fail -> 1
        is Test451State.P -> 2
        is Test451State.Pass -> 0
        is Test451State.S0 -> 3
        is Test451State.S1 -> 4
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test451State,
        event: Test451Event
    ): TransitionResult<Test451State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test451State
    ): TransitionResult<Test451State> = when (state) {
        is Test451State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test451State> = when {
        isStateActive("s1") -> TransitionResult.External(Test451State.Pass, Test451State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test451State.Fail, Test451State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test451.scxml:5
    override fun onEntry(state: Test451State) {
        when (state) {
            is Test451State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test451State.P -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test451State.S0)
                onEntry(Test451State.S1)
            }
            is Test451State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test451State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test451State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test451.scxml:5
    override fun onExit(state: Test451State) {
        when (state) {
            is Test451State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test451State.P -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test451State, Int>>()
                if (activeStateIds.contains("s0")) {
                    toExit.add(Test451State.S0 to 3)
                }
                if (activeStateIds.contains("s1")) {
                    toExit.add(Test451State.S1 to 4)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p")
            }
            is Test451State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test451State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test451State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test451.scxml:5
    override fun executeTransitionActions(
        source: Test451State,
        event: Test451Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
