// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8935a2b1ceca80a03ff3489cc9f8dcbccd8c2b85fc58c3b848403d6a2672153
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/576/test576.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test576.scxml:6 :: _machine

package com.sce.generated.test576

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test576State : State {
    data object Fail : Test576State
    data object Pass : Test576State
    data object S0 : Test576State
    data object S1 : Test576State
    data object S11 : Test576State
    data object S111 : Test576State
    data object S11p1 : Test576State
    data object S11p11 : Test576State
    data object S11p111 : Test576State
    data object S11p112 : Test576State
    data object S11p12 : Test576State
    data object S11p121 : Test576State
    data object S11p122 : Test576State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test576Event : Event {
    data object InS11p112 : Test576Event
    sealed interface Error : Test576Event {
        data object Execution : Error
    }
    data object Timeout : Test576Event
}
// --- State Machine (W3C SCXML) ---

class Test576StateMachine(
) : StateMachineEngine<Test576State, Test576Event>() {

    override val initialState: Test576State = Test576State.S11p112

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test576State): Test576State? = when (state) {
        is Test576State.S11 -> Test576State.S1
        is Test576State.S111 -> Test576State.S11
        is Test576State.S11p1 -> Test576State.S11
        is Test576State.S11p11 -> Test576State.S11p1
        is Test576State.S11p111 -> Test576State.S11p11
        is Test576State.S11p112 -> Test576State.S11p11
        is Test576State.S11p12 -> Test576State.S11p1
        is Test576State.S11p121 -> Test576State.S11p12
        is Test576State.S11p122 -> Test576State.S11p12
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test576State): Test576State = when (state) {
        is Test576State.S1 -> Test576State.S11p112
        is Test576State.S11 -> Test576State.S11p112
        is Test576State.S11p1 -> Test576State.S11p112
        is Test576State.S11p11 -> Test576State.S11p112
        is Test576State.S11p12 -> Test576State.S11p122
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test576State? = when (stateId) {
        "fail" -> Test576State.Fail
        "pass" -> Test576State.Pass
        "s0" -> Test576State.S0
        "s1" -> Test576State.S1
        "s11" -> Test576State.S11
        "s111" -> Test576State.S111
        "s11p1" -> Test576State.S11p1
        "s11p11" -> Test576State.S11p11
        "s11p111" -> Test576State.S11p111
        "s11p112" -> Test576State.S11p112
        "s11p12" -> Test576State.S11p12
        "s11p121" -> Test576State.S11p121
        "s11p122" -> Test576State.S11p122
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test576State): String = when (state) {
        is Test576State.Fail -> "fail"
        is Test576State.Pass -> "pass"
        is Test576State.S0 -> "s0"
        is Test576State.S1 -> "s1"
        is Test576State.S11 -> "s11"
        is Test576State.S111 -> "s111"
        is Test576State.S11p1 -> "s11p1"
        is Test576State.S11p11 -> "s11p11"
        is Test576State.S11p111 -> "s11p111"
        is Test576State.S11p112 -> "s11p112"
        is Test576State.S11p12 -> "s11p12"
        is Test576State.S11p121 -> "s11p121"
        is Test576State.S11p122 -> "s11p122"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test576State): Boolean = when (state) {
        is Test576State.S1 -> false
        is Test576State.S11 -> false
        is Test576State.S11p1 -> false
        is Test576State.S11p11 -> false
        is Test576State.S11p12 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test576State): Boolean = when (state) {
        is Test576State.S11p1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test576State): List<Test576State> = when (state) {
        is Test576State.S11p1 -> listOf(Test576State.S11p11, Test576State.S11p12)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test576State): Int = when (state) {
        is Test576State.Fail -> 12
        is Test576State.Pass -> 11
        is Test576State.S0 -> 0
        is Test576State.S1 -> 1
        is Test576State.S11 -> 2
        is Test576State.S111 -> 3
        is Test576State.S11p1 -> 4
        is Test576State.S11p11 -> 5
        is Test576State.S11p111 -> 6
        is Test576State.S11p112 -> 7
        is Test576State.S11p12 -> 8
        is Test576State.S11p121 -> 9
        is Test576State.S11p122 -> 10
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test576State,
        event: Test576Event
    ): TransitionResult<Test576State> = when (state) {
        is Test576State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test576State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test576State.S111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p11 has no own event transitions)
        is Test576State.S11p11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p111 has no own event transitions)
        is Test576State.S11p111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p112 has no own event transitions)
        is Test576State.S11p112 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p12 has no own event transitions)
        is Test576State.S11p12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p121 has no own event transitions)
        is Test576State.S11p121 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test576State.S11p122 -> {
            val result = processS11p122(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test576State
    ): TransitionResult<Test576State> = when (state) {
        is Test576State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test576State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test576State.Fail, Test576State.S0)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test576Event
    ): TransitionResult<Test576State> = when {
        event is Test576Event.Timeout -> TransitionResult.External(Test576State.Fail, Test576State.S1)

        else -> TransitionResult.Ignored
    }

    private fun processS11p122(
        event: Test576Event
    ): TransitionResult<Test576State> = when {
        event is Test576Event.InS11p112 -> TransitionResult.External(Test576State.Pass, Test576State.S11p122)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun onEntry(state: Test576State) {
        when (state) {
            is Test576State.Fail -> {
                // SCE-MAP: test576.scxml:40 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test576State.Pass -> {
                // SCE-MAP: test576.scxml:39 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test576State.S0 -> {
                // SCE-MAP: test576.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test576State.S1 -> {
                // SCE-MAP: test576.scxml:13 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_0", 1000L, Test576Event.Timeout)
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test576State.S11)
                }
            }
            is Test576State.S11 -> {
                // SCE-MAP: test576.scxml:18 :: s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test576State.S11p1)
                }
            }
            is Test576State.S111 -> {
                // SCE-MAP: test576.scxml:19 :: s111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s111")) return
            }
            is Test576State.S11p1 -> {
                // SCE-MAP: test576.scxml:20 :: s11p1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p1")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test576State.S11p11)
                onEntry(Test576State.S11p12)
            }
            is Test576State.S11p11 -> {
                // SCE-MAP: test576.scxml:21 :: s11p11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p11")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test576State.S11p112)
                }
            }
            is Test576State.S11p111 -> {
                // SCE-MAP: test576.scxml:22 :: s11p111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p111")) return
            }
            is Test576State.S11p112 -> {
                // SCE-MAP: test576.scxml:23 :: s11p112 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p112")) return

            raiseInternal(Test576Event.InS11p112)
            }
            is Test576State.S11p12 -> {
                // SCE-MAP: test576.scxml:29 :: s11p12 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p12")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test576State.S11p122)
                }
            }
            is Test576State.S11p121 -> {
                // SCE-MAP: test576.scxml:30 :: s11p121 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p121")) return
            }
            is Test576State.S11p122 -> {
                // SCE-MAP: test576.scxml:31 :: s11p122 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p122")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun onExit(state: Test576State) {
        when (state) {
            is Test576State.Fail -> {
                // SCE-MAP: test576.scxml:40 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test576State.Pass -> {
                // SCE-MAP: test576.scxml:39 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test576State.S0 -> {
                // SCE-MAP: test576.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test576State.S1 -> {
                // SCE-MAP: test576.scxml:13 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test576State.S11 -> {
                // SCE-MAP: test576.scxml:18 :: s11 :: _state_body
                activeStateIds.remove("s11")
            }
            is Test576State.S111 -> {
                // SCE-MAP: test576.scxml:19 :: s111 :: _state_body
                activeStateIds.remove("s111")
            }
            is Test576State.S11p1 -> {
                // SCE-MAP: test576.scxml:20 :: s11p1 :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test576State, Int>>()
                if (activeStateIds.contains("s11p11")) {
                    toExit.add(Test576State.S11p11 to 5)
                }
                if (activeStateIds.contains("s11p111")) {
                    toExit.add(Test576State.S11p111 to 6)
                }
                if (activeStateIds.contains("s11p112")) {
                    toExit.add(Test576State.S11p112 to 7)
                }
                if (activeStateIds.contains("s11p12")) {
                    toExit.add(Test576State.S11p12 to 8)
                }
                if (activeStateIds.contains("s11p121")) {
                    toExit.add(Test576State.S11p121 to 9)
                }
                if (activeStateIds.contains("s11p122")) {
                    toExit.add(Test576State.S11p122 to 10)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s11p1")
            }
            is Test576State.S11p11 -> {
                // SCE-MAP: test576.scxml:21 :: s11p11 :: _state_body
                activeStateIds.remove("s11p11")
            }
            is Test576State.S11p111 -> {
                // SCE-MAP: test576.scxml:22 :: s11p111 :: _state_body
                activeStateIds.remove("s11p111")
            }
            is Test576State.S11p112 -> {
                // SCE-MAP: test576.scxml:23 :: s11p112 :: _state_body
                activeStateIds.remove("s11p112")
            }
            is Test576State.S11p12 -> {
                // SCE-MAP: test576.scxml:29 :: s11p12 :: _state_body
                activeStateIds.remove("s11p12")
            }
            is Test576State.S11p121 -> {
                // SCE-MAP: test576.scxml:30 :: s11p121 :: _state_body
                activeStateIds.remove("s11p121")
            }
            is Test576State.S11p122 -> {
                // SCE-MAP: test576.scxml:31 :: s11p122 :: _state_body
                activeStateIds.remove("s11p122")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test576State,
        event: Test576Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
