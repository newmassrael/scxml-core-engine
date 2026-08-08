// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/436/test436.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test436.scxml:3

package com.sce.generated.test436

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test436State : State {
    data object Fail : Test436State
    data object P : Test436State
    data object Pass : Test436State
    data object Ps0 : Test436State
    data object Ps1 : Test436State
    data object S1 : Test436State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test436Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test436StateMachine(
) : StateMachineEngine<Test436State, Test436Event>() {

    override val initialState: Test436State = Test436State.Ps0

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test436State): Test436State? = when (state) {
        is Test436State.Ps0 -> Test436State.P
        is Test436State.Ps1 -> Test436State.P
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test436State): Test436State = when (state) {
        is Test436State.P -> Test436State.Ps0
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test436State? = when (stateId) {
        "fail" -> Test436State.Fail
        "p" -> Test436State.P
        "pass" -> Test436State.Pass
        "ps0" -> Test436State.Ps0
        "ps1" -> Test436State.Ps1
        "s1" -> Test436State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test436State): String = when (state) {
        is Test436State.Fail -> "fail"
        is Test436State.P -> "p"
        is Test436State.Pass -> "pass"
        is Test436State.Ps0 -> "ps0"
        is Test436State.Ps1 -> "ps1"
        is Test436State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test436State): Boolean = when (state) {
        is Test436State.P -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test436State): Boolean = when (state) {
        is Test436State.P -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test436State): List<Test436State> = when (state) {
        is Test436State.P -> listOf(Test436State.Ps0, Test436State.Ps1)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test436State): Int = when (state) {
        is Test436State.Fail -> 5
        is Test436State.P -> 0
        is Test436State.Pass -> 4
        is Test436State.Ps0 -> 1
        is Test436State.Ps1 -> 2
        is Test436State.S1 -> 3
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test436State,
        event: Test436Event
    ): TransitionResult<Test436State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test436State
    ): TransitionResult<Test436State> = when (state) {
        is Test436State.Ps0 -> processNullPs0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullPs0(
    ): TransitionResult<Test436State> = when {
        isStateActive("s1") -> TransitionResult.External(Test436State.Fail, Test436State.Ps0)
        isStateActive("ps1") -> TransitionResult.External(Test436State.Pass, Test436State.Ps0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test436State.Fail, Test436State.Ps0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test436.scxml:3
    override fun onEntry(state: Test436State) {
        when (state) {
            is Test436State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test436State.P -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test436State.Ps0)
                onEntry(Test436State.Ps1)
            }
            is Test436State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test436State.Ps0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps0")) return
            }
            is Test436State.Ps1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps1")) return
            }
            is Test436State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test436.scxml:3
    override fun onExit(state: Test436State) {
        when (state) {
            is Test436State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test436State.P -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test436State, Int>>()
                if (activeStateIds.contains("ps0")) {
                    toExit.add(Test436State.Ps0 to 1)
                }
                if (activeStateIds.contains("ps1")) {
                    toExit.add(Test436State.Ps1 to 2)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p")
            }
            is Test436State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test436State.Ps0 -> {
                activeStateIds.remove("ps0")
            }
            is Test436State.Ps1 -> {
                activeStateIds.remove("ps1")
            }
            is Test436State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test436.scxml:3
    override fun executeTransitionActions(
        source: Test436State,
        event: Test436Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
