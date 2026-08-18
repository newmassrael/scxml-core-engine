// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/364/test364.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test364.scxml:7 :: _machine

package com.sce.generated.test364

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test364State : State {
    data object Fail : Test364State
    data object Pass : Test364State
    data object S1 : Test364State
    data object S11 : Test364State
    data object S111 : Test364State
    data object S11p1 : Test364State
    data object S11p11 : Test364State
    data object S11p111 : Test364State
    data object S11p112 : Test364State
    data object S11p12 : Test364State
    data object S11p121 : Test364State
    data object S11p122 : Test364State
    data object S2 : Test364State
    data object S21 : Test364State
    data object S211 : Test364State
    data object S21p1 : Test364State
    data object S21p11 : Test364State
    data object S21p111 : Test364State
    data object S21p112 : Test364State
    data object S21p12 : Test364State
    data object S21p121 : Test364State
    data object S21p122 : Test364State
    data object S3 : Test364State
    data object S31 : Test364State
    data object S311 : Test364State
    data object S3111 : Test364State
    data object S3112 : Test364State
    data object S312 : Test364State
    data object S32 : Test364State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test364Event : Event {
    data object InS11p112 : Test364Event
    data object InS21p112 : Test364Event
    sealed interface Error : Test364Event {
        data object Execution : Error
    }
    data object Timeout : Test364Event
}
// --- State Machine (W3C SCXML) ---

class Test364StateMachine(
) : StateMachineEngine<Test364State, Test364Event>() {

    override val initialState: Test364State = Test364State.S1

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test364State): Test364State? = when (state) {
        is Test364State.S11 -> Test364State.S1
        is Test364State.S111 -> Test364State.S11
        is Test364State.S11p1 -> Test364State.S11
        is Test364State.S11p11 -> Test364State.S11p1
        is Test364State.S11p111 -> Test364State.S11p11
        is Test364State.S11p112 -> Test364State.S11p11
        is Test364State.S11p12 -> Test364State.S11p1
        is Test364State.S11p121 -> Test364State.S11p12
        is Test364State.S11p122 -> Test364State.S11p12
        is Test364State.S21 -> Test364State.S2
        is Test364State.S211 -> Test364State.S21
        is Test364State.S21p1 -> Test364State.S21
        is Test364State.S21p11 -> Test364State.S21p1
        is Test364State.S21p111 -> Test364State.S21p11
        is Test364State.S21p112 -> Test364State.S21p11
        is Test364State.S21p12 -> Test364State.S21p1
        is Test364State.S21p121 -> Test364State.S21p12
        is Test364State.S21p122 -> Test364State.S21p12
        is Test364State.S31 -> Test364State.S3
        is Test364State.S311 -> Test364State.S31
        is Test364State.S3111 -> Test364State.S311
        is Test364State.S3112 -> Test364State.S311
        is Test364State.S312 -> Test364State.S311
        is Test364State.S32 -> Test364State.S311
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test364State): Test364State = when (state) {
        is Test364State.S11 -> Test364State.S11p112
        is Test364State.S11p1 -> Test364State.S11p112
        is Test364State.S11p11 -> Test364State.S11p112
        is Test364State.S11p12 -> Test364State.S11p122
        is Test364State.S21 -> Test364State.S21p112
        is Test364State.S21p1 -> Test364State.S21p112
        is Test364State.S21p11 -> Test364State.S21p112
        is Test364State.S21p12 -> Test364State.S21p122
        is Test364State.S3 -> Test364State.S3111
        is Test364State.S31 -> Test364State.S3111
        is Test364State.S311 -> Test364State.S3111
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test364State? = when (stateId) {
        "fail" -> Test364State.Fail
        "pass" -> Test364State.Pass
        "s1" -> Test364State.S1
        "s11" -> Test364State.S11
        "s111" -> Test364State.S111
        "s11p1" -> Test364State.S11p1
        "s11p11" -> Test364State.S11p11
        "s11p111" -> Test364State.S11p111
        "s11p112" -> Test364State.S11p112
        "s11p12" -> Test364State.S11p12
        "s11p121" -> Test364State.S11p121
        "s11p122" -> Test364State.S11p122
        "s2" -> Test364State.S2
        "s21" -> Test364State.S21
        "s211" -> Test364State.S211
        "s21p1" -> Test364State.S21p1
        "s21p11" -> Test364State.S21p11
        "s21p111" -> Test364State.S21p111
        "s21p112" -> Test364State.S21p112
        "s21p12" -> Test364State.S21p12
        "s21p121" -> Test364State.S21p121
        "s21p122" -> Test364State.S21p122
        "s3" -> Test364State.S3
        "s31" -> Test364State.S31
        "s311" -> Test364State.S311
        "s3111" -> Test364State.S3111
        "s3112" -> Test364State.S3112
        "s312" -> Test364State.S312
        "s32" -> Test364State.S32
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test364State): String = when (state) {
        is Test364State.Fail -> "fail"
        is Test364State.Pass -> "pass"
        is Test364State.S1 -> "s1"
        is Test364State.S11 -> "s11"
        is Test364State.S111 -> "s111"
        is Test364State.S11p1 -> "s11p1"
        is Test364State.S11p11 -> "s11p11"
        is Test364State.S11p111 -> "s11p111"
        is Test364State.S11p112 -> "s11p112"
        is Test364State.S11p12 -> "s11p12"
        is Test364State.S11p121 -> "s11p121"
        is Test364State.S11p122 -> "s11p122"
        is Test364State.S2 -> "s2"
        is Test364State.S21 -> "s21"
        is Test364State.S211 -> "s211"
        is Test364State.S21p1 -> "s21p1"
        is Test364State.S21p11 -> "s21p11"
        is Test364State.S21p111 -> "s21p111"
        is Test364State.S21p112 -> "s21p112"
        is Test364State.S21p12 -> "s21p12"
        is Test364State.S21p121 -> "s21p121"
        is Test364State.S21p122 -> "s21p122"
        is Test364State.S3 -> "s3"
        is Test364State.S31 -> "s31"
        is Test364State.S311 -> "s311"
        is Test364State.S3111 -> "s3111"
        is Test364State.S3112 -> "s3112"
        is Test364State.S312 -> "s312"
        is Test364State.S32 -> "s32"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test364State): Boolean = when (state) {
        is Test364State.S1 -> false
        is Test364State.S11 -> false
        is Test364State.S11p1 -> false
        is Test364State.S11p11 -> false
        is Test364State.S11p12 -> false
        is Test364State.S2 -> false
        is Test364State.S21 -> false
        is Test364State.S21p1 -> false
        is Test364State.S21p11 -> false
        is Test364State.S21p12 -> false
        is Test364State.S3 -> false
        is Test364State.S31 -> false
        is Test364State.S311 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test364State): Boolean = when (state) {
        is Test364State.S11p1 -> true
        is Test364State.S21p1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test364State): List<Test364State> = when (state) {
        is Test364State.S11p1 -> listOf(Test364State.S11p11, Test364State.S11p12)
        is Test364State.S21p1 -> listOf(Test364State.S21p11, Test364State.S21p12)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test364State): Int = when (state) {
        is Test364State.Fail -> 28
        is Test364State.Pass -> 27
        is Test364State.S1 -> 0
        is Test364State.S11 -> 1
        is Test364State.S111 -> 2
        is Test364State.S11p1 -> 3
        is Test364State.S11p11 -> 4
        is Test364State.S11p111 -> 5
        is Test364State.S11p112 -> 6
        is Test364State.S11p12 -> 7
        is Test364State.S11p121 -> 8
        is Test364State.S11p122 -> 9
        is Test364State.S2 -> 10
        is Test364State.S21 -> 11
        is Test364State.S211 -> 12
        is Test364State.S21p1 -> 13
        is Test364State.S21p11 -> 14
        is Test364State.S21p111 -> 15
        is Test364State.S21p112 -> 16
        is Test364State.S21p12 -> 17
        is Test364State.S21p121 -> 18
        is Test364State.S21p122 -> 19
        is Test364State.S3 -> 20
        is Test364State.S31 -> 21
        is Test364State.S311 -> 22
        is Test364State.S3111 -> 23
        is Test364State.S3112 -> 24
        is Test364State.S312 -> 25
        is Test364State.S32 -> 26
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test364State,
        event: Test364Event
    ): TransitionResult<Test364State> = when (state) {
        is Test364State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test364State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test364State.S111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p11 has no own event transitions)
        is Test364State.S11p11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p111 has no own event transitions)
        is Test364State.S11p111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p112 has no own event transitions)
        is Test364State.S11p112 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p12 has no own event transitions)
        is Test364State.S11p12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11p121 has no own event transitions)
        is Test364State.S11p121 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test364State.S11p122 -> {
            val result = processS11p122(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test364State.S2 -> processS2(event)
        // W3C SCXML 3.13: Ancestor-only routing (s21 has no own event transitions)
        is Test364State.S21 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s211 has no own event transitions)
        is Test364State.S211 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s21p11 has no own event transitions)
        is Test364State.S21p11 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s21p111 has no own event transitions)
        is Test364State.S21p111 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s21p112 has no own event transitions)
        is Test364State.S21p112 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s21p12 has no own event transitions)
        is Test364State.S21p12 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s21p121 has no own event transitions)
        is Test364State.S21p121 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test364State.S21p122 -> {
            val result = processS21p122(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS2(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test364State
    ): TransitionResult<Test364State> = when (state) {
        is Test364State.S3 -> processNullS3()
        is Test364State.S31 -> processNullS3()
        is Test364State.S311 -> processNullS3()
        is Test364State.S3111 -> {
            val null1 = processNullS3111()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS3()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        is Test364State.S3112 -> processNullS3()
        is Test364State.S312 -> processNullS3()
        is Test364State.S32 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS3(
    ): TransitionResult<Test364State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test364State.Fail, Test364State.S3)
    }

    private fun processNullS3111(
    ): TransitionResult<Test364State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test364State.Pass, Test364State.S3111)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test364Event
    ): TransitionResult<Test364State> = when {
        event is Test364Event.Timeout -> TransitionResult.External(Test364State.Fail, Test364State.S1)

        else -> TransitionResult.Ignored
    }

    private fun processS11p122(
        event: Test364Event
    ): TransitionResult<Test364State> = when {
        event is Test364Event.InS11p112 -> TransitionResult.External(Test364State.S2, Test364State.S11p122)

        else -> TransitionResult.Ignored
    }

    private fun processS2(
        event: Test364Event
    ): TransitionResult<Test364State> = when {
        event is Test364Event.Timeout -> TransitionResult.External(Test364State.Fail, Test364State.S2)

        else -> TransitionResult.Ignored
    }

    private fun processS21p122(
        event: Test364Event
    ): TransitionResult<Test364State> = when {
        event is Test364Event.InS21p112 -> TransitionResult.External(Test364State.S3, Test364State.S21p122)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun onEntry(state: Test364State, pathChild: Test364State?) {
        when (state) {
            is Test364State.Fail -> {
                // SCE-MAP: test364.scxml:76 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test364State.Pass -> {
                // SCE-MAP: test364.scxml:75 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test364State.S1 -> {
                // SCE-MAP: test364.scxml:9 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_0", 1000L, Test364Event.Timeout)
                if (pathChild == null) {
                    // W3C SCXML 3.6: Enter deep initial targets (C++ enterDeepInitialTargets pattern)
                    onEntry(Test364State.S11)
                    onEntry(Test364State.S11p1)
                    onEntry(Test364State.S11p11)
                    onEntry(Test364State.S11p112)
                    onEntry(Test364State.S11p12)
                    onEntry(Test364State.S11p122)
                }
            }
            is Test364State.S11 -> {
                // SCE-MAP: test364.scxml:14 :: s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S11p1)
                }
            }
            is Test364State.S111 -> {
                // SCE-MAP: test364.scxml:15 :: s111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s111")) return
            }
            is Test364State.S11p1 -> {
                // SCE-MAP: test364.scxml:16 :: s11p1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p1")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test364State.S11p11) {
                    onEntry(Test364State.S11p11)
                }
                if (pathChild != Test364State.S11p12) {
                    onEntry(Test364State.S11p12)
                }
            }
            is Test364State.S11p11 -> {
                // SCE-MAP: test364.scxml:17 :: s11p11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p11")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S11p112)
                }
            }
            is Test364State.S11p111 -> {
                // SCE-MAP: test364.scxml:18 :: s11p111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p111")) return
            }
            is Test364State.S11p112 -> {
                // SCE-MAP: test364.scxml:19 :: s11p112 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p112")) return

            raiseInternal(Test364Event.InS11p112)
            }
            is Test364State.S11p12 -> {
                // SCE-MAP: test364.scxml:25 :: s11p12 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p12")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S11p122)
                }
            }
            is Test364State.S11p121 -> {
                // SCE-MAP: test364.scxml:26 :: s11p121 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p121")) return
            }
            is Test364State.S11p122 -> {
                // SCE-MAP: test364.scxml:27 :: s11p122 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11p122")) return
            }
            is Test364State.S2 -> {
                // SCE-MAP: test364.scxml:35 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
                if (pathChild == null) {
                    // W3C SCXML 3.6: Enter deep initial targets (C++ enterDeepInitialTargets pattern)
                    onEntry(Test364State.S21)
                    onEntry(Test364State.S21p1)
                    onEntry(Test364State.S21p11)
                    onEntry(Test364State.S21p112)
                    onEntry(Test364State.S21p12)
                    onEntry(Test364State.S21p122)
                }
            }
            is Test364State.S21 -> {
                // SCE-MAP: test364.scxml:40 :: s21 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S21p1)
                }
            }
            is Test364State.S211 -> {
                // SCE-MAP: test364.scxml:41 :: s211 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s211")) return
            }
            is Test364State.S21p1 -> {
                // SCE-MAP: test364.scxml:42 :: s21p1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p1")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test364State.S21p11) {
                    onEntry(Test364State.S21p11)
                }
                if (pathChild != Test364State.S21p12) {
                    onEntry(Test364State.S21p12)
                }
            }
            is Test364State.S21p11 -> {
                // SCE-MAP: test364.scxml:43 :: s21p11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p11")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S21p112)
                }
            }
            is Test364State.S21p111 -> {
                // SCE-MAP: test364.scxml:44 :: s21p111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p111")) return
            }
            is Test364State.S21p112 -> {
                // SCE-MAP: test364.scxml:45 :: s21p112 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p112")) return

            raiseInternal(Test364Event.InS21p112)
            }
            is Test364State.S21p12 -> {
                // SCE-MAP: test364.scxml:51 :: s21p12 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p12")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S21p122)
                }
            }
            is Test364State.S21p121 -> {
                // SCE-MAP: test364.scxml:52 :: s21p121 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p121")) return
            }
            is Test364State.S21p122 -> {
                // SCE-MAP: test364.scxml:53 :: s21p122 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21p122")) return
            }
            is Test364State.S3 -> {
                // SCE-MAP: test364.scxml:61 :: s3 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S31)
                }
            }
            is Test364State.S31 -> {
                // SCE-MAP: test364.scxml:63 :: s31 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s31")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S311)
                }
            }
            is Test364State.S311 -> {
                // SCE-MAP: test364.scxml:64 :: s311 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s311")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test364State.S3111)
                }
            }
            is Test364State.S3111 -> {
                // SCE-MAP: test364.scxml:65 :: s3111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3111")) return
            }
            is Test364State.S3112 -> {
                // SCE-MAP: test364.scxml:68 :: s3112 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3112")) return
            }
            is Test364State.S312 -> {
                // SCE-MAP: test364.scxml:69 :: s312 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s312")) return
            }
            is Test364State.S32 -> {
                // SCE-MAP: test364.scxml:70 :: s32 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s32")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun onExit(state: Test364State) {
        when (state) {
            is Test364State.Fail -> {
                // SCE-MAP: test364.scxml:76 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test364State.Pass -> {
                // SCE-MAP: test364.scxml:75 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test364State.S1 -> {
                // SCE-MAP: test364.scxml:9 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test364State.S11 -> {
                // SCE-MAP: test364.scxml:14 :: s11 :: _state_body
                activeStateIds.remove("s11")
            }
            is Test364State.S111 -> {
                // SCE-MAP: test364.scxml:15 :: s111 :: _state_body
                activeStateIds.remove("s111")
            }
            is Test364State.S11p1 -> {
                // SCE-MAP: test364.scxml:16 :: s11p1 :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test364State, Int>>()
                if (activeStateIds.contains("s11p11")) {
                    toExit.add(Test364State.S11p11 to 4)
                }
                if (activeStateIds.contains("s11p111")) {
                    toExit.add(Test364State.S11p111 to 5)
                }
                if (activeStateIds.contains("s11p112")) {
                    toExit.add(Test364State.S11p112 to 6)
                }
                if (activeStateIds.contains("s11p12")) {
                    toExit.add(Test364State.S11p12 to 7)
                }
                if (activeStateIds.contains("s11p121")) {
                    toExit.add(Test364State.S11p121 to 8)
                }
                if (activeStateIds.contains("s11p122")) {
                    toExit.add(Test364State.S11p122 to 9)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s11p1")
            }
            is Test364State.S11p11 -> {
                // SCE-MAP: test364.scxml:17 :: s11p11 :: _state_body
                activeStateIds.remove("s11p11")
            }
            is Test364State.S11p111 -> {
                // SCE-MAP: test364.scxml:18 :: s11p111 :: _state_body
                activeStateIds.remove("s11p111")
            }
            is Test364State.S11p112 -> {
                // SCE-MAP: test364.scxml:19 :: s11p112 :: _state_body
                activeStateIds.remove("s11p112")
            }
            is Test364State.S11p12 -> {
                // SCE-MAP: test364.scxml:25 :: s11p12 :: _state_body
                activeStateIds.remove("s11p12")
            }
            is Test364State.S11p121 -> {
                // SCE-MAP: test364.scxml:26 :: s11p121 :: _state_body
                activeStateIds.remove("s11p121")
            }
            is Test364State.S11p122 -> {
                // SCE-MAP: test364.scxml:27 :: s11p122 :: _state_body
                activeStateIds.remove("s11p122")
            }
            is Test364State.S2 -> {
                // SCE-MAP: test364.scxml:35 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
            is Test364State.S21 -> {
                // SCE-MAP: test364.scxml:40 :: s21 :: _state_body
                activeStateIds.remove("s21")
            }
            is Test364State.S211 -> {
                // SCE-MAP: test364.scxml:41 :: s211 :: _state_body
                activeStateIds.remove("s211")
            }
            is Test364State.S21p1 -> {
                // SCE-MAP: test364.scxml:42 :: s21p1 :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test364State, Int>>()
                if (activeStateIds.contains("s21p11")) {
                    toExit.add(Test364State.S21p11 to 14)
                }
                if (activeStateIds.contains("s21p111")) {
                    toExit.add(Test364State.S21p111 to 15)
                }
                if (activeStateIds.contains("s21p112")) {
                    toExit.add(Test364State.S21p112 to 16)
                }
                if (activeStateIds.contains("s21p12")) {
                    toExit.add(Test364State.S21p12 to 17)
                }
                if (activeStateIds.contains("s21p121")) {
                    toExit.add(Test364State.S21p121 to 18)
                }
                if (activeStateIds.contains("s21p122")) {
                    toExit.add(Test364State.S21p122 to 19)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s21p1")
            }
            is Test364State.S21p11 -> {
                // SCE-MAP: test364.scxml:43 :: s21p11 :: _state_body
                activeStateIds.remove("s21p11")
            }
            is Test364State.S21p111 -> {
                // SCE-MAP: test364.scxml:44 :: s21p111 :: _state_body
                activeStateIds.remove("s21p111")
            }
            is Test364State.S21p112 -> {
                // SCE-MAP: test364.scxml:45 :: s21p112 :: _state_body
                activeStateIds.remove("s21p112")
            }
            is Test364State.S21p12 -> {
                // SCE-MAP: test364.scxml:51 :: s21p12 :: _state_body
                activeStateIds.remove("s21p12")
            }
            is Test364State.S21p121 -> {
                // SCE-MAP: test364.scxml:52 :: s21p121 :: _state_body
                activeStateIds.remove("s21p121")
            }
            is Test364State.S21p122 -> {
                // SCE-MAP: test364.scxml:53 :: s21p122 :: _state_body
                activeStateIds.remove("s21p122")
            }
            is Test364State.S3 -> {
                // SCE-MAP: test364.scxml:61 :: s3 :: _state_body
                activeStateIds.remove("s3")
            }
            is Test364State.S31 -> {
                // SCE-MAP: test364.scxml:63 :: s31 :: _state_body
                activeStateIds.remove("s31")
            }
            is Test364State.S311 -> {
                // SCE-MAP: test364.scxml:64 :: s311 :: _state_body
                activeStateIds.remove("s311")
            }
            is Test364State.S3111 -> {
                // SCE-MAP: test364.scxml:65 :: s3111 :: _state_body
                activeStateIds.remove("s3111")
            }
            is Test364State.S3112 -> {
                // SCE-MAP: test364.scxml:68 :: s3112 :: _state_body
                activeStateIds.remove("s3112")
            }
            is Test364State.S312 -> {
                // SCE-MAP: test364.scxml:69 :: s312 :: _state_body
                activeStateIds.remove("s312")
            }
            is Test364State.S32 -> {
                // SCE-MAP: test364.scxml:70 :: s32 :: _state_body
                activeStateIds.remove("s32")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun executeTransitionActions(
        source: Test364State,
        event: Test364Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
