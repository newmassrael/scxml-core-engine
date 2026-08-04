// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 817d9c061804919d9748138703a11f334a156a4e2a1e5a3c66f1c4e7ca554aa2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/310/test310.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test310.scxml:5

package com.sce.generated.test310

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test310State : State {
    data object Fail : Test310State
    data object P : Test310State
    data object Pass : Test310State
    data object S0 : Test310State
    data object S1 : Test310State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test310Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test310StateMachine(
) : StateMachineEngine<Test310State, Test310Event>() {

    override val initialState: Test310State = Test310State.S0

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test310State): Test310State? = when (state) {
        is Test310State.S0 -> Test310State.P
        is Test310State.S1 -> Test310State.P
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test310State): Test310State = when (state) {
        is Test310State.P -> Test310State.S0
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test310State? = when (stateId) {
        "fail" -> Test310State.Fail
        "p" -> Test310State.P
        "pass" -> Test310State.Pass
        "s0" -> Test310State.S0
        "s1" -> Test310State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test310State): String = when (state) {
        is Test310State.Fail -> "fail"
        is Test310State.P -> "p"
        is Test310State.Pass -> "pass"
        is Test310State.S0 -> "s0"
        is Test310State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test310State): Boolean = when (state) {
        is Test310State.P -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test310State): Boolean = when (state) {
        is Test310State.P -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test310State): List<Test310State> = when (state) {
        is Test310State.P -> listOf(Test310State.S0, Test310State.S1)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test310State): Int = when (state) {
        is Test310State.Fail -> 4
        is Test310State.P -> 0
        is Test310State.Pass -> 3
        is Test310State.S0 -> 1
        is Test310State.S1 -> 2
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test310State,
        event: Test310Event
    ): TransitionResult<Test310State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test310State
    ): TransitionResult<Test310State> = when (state) {
        is Test310State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test310State> = when {
        isStateActive("s1") -> TransitionResult.External(Test310State.Pass, Test310State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test310State.Fail, Test310State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test310.scxml:5
    override fun onEntry(state: Test310State) {
        when (state) {
            is Test310State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test310State.P -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test310State.S0)
                onEntry(Test310State.S1)
            }
            is Test310State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test310State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test310State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test310.scxml:5
    override fun onExit(state: Test310State) {
        when (state) {
            is Test310State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test310State.P -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test310State, Int>>()
                if (activeStateIds.contains("s0")) {
                    toExit.add(Test310State.S0 to 1)
                }
                if (activeStateIds.contains("s1")) {
                    toExit.add(Test310State.S1 to 2)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p")
            }
            is Test310State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test310State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test310State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test310.scxml:5
    override fun executeTransitionActions(
        source: Test310State,
        event: Test310Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
