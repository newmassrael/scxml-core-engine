// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712

// GENERATED CODE — DO NOT EDIT
// Source: resources/387/test387.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test387.scxml:7

package com.sce.generated.test387

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test387State : State {
    data object Fail : Test387State
    data object Pass : Test387State
    data object S0 : Test387State
    data object S01 : Test387State
    data object S011 : Test387State
    data object S012 : Test387State
    data object S02 : Test387State
    data object S021 : Test387State
    data object S022 : Test387State
    data object S1 : Test387State
    data object S11 : Test387State
    data object S111 : Test387State
    data object S112 : Test387State
    data object S12 : Test387State
    data object S121 : Test387State
    data object S122 : Test387State
    data object S3 : Test387State
    data object S4 : Test387State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test387Event : Event {
    data object EnteringS011 : Test387Event
    data object EnteringS012 : Test387Event
    data object EnteringS021 : Test387Event
    data object EnteringS022 : Test387Event
    data object EnteringS111 : Test387Event
    data object EnteringS112 : Test387Event
    data object EnteringS121 : Test387Event
    data object EnteringS122 : Test387Event
    sealed interface Error : Test387Event {
        data object Execution : Error
    }
    data object Timeout : Test387Event
}
// --- State Machine (W3C SCXML) ---

class Test387StateMachine(
) : StateMachineEngine<Test387State, Test387Event>() {

    override val initialState: Test387State = Test387State.S3

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test387State): Test387State? = when (state) {
        is Test387State.S01 -> Test387State.S0
        is Test387State.S011 -> Test387State.S01
        is Test387State.S012 -> Test387State.S01
        is Test387State.S02 -> Test387State.S0
        is Test387State.S021 -> Test387State.S02
        is Test387State.S022 -> Test387State.S02
        is Test387State.S11 -> Test387State.S1
        is Test387State.S111 -> Test387State.S11
        is Test387State.S112 -> Test387State.S11
        is Test387State.S12 -> Test387State.S1
        is Test387State.S121 -> Test387State.S12
        is Test387State.S122 -> Test387State.S12
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test387State): Test387State = when (state) {
        is Test387State.S0 -> Test387State.S011
        is Test387State.S01 -> Test387State.S011
        is Test387State.S02 -> Test387State.S021
        is Test387State.S1 -> Test387State.S111
        is Test387State.S11 -> Test387State.S111
        is Test387State.S12 -> Test387State.S121
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test387State? = when (stateId) {
        "fail" -> Test387State.Fail
        "pass" -> Test387State.Pass
        "s0" -> Test387State.S0
        "s01" -> Test387State.S01
        "s011" -> Test387State.S011
        "s012" -> Test387State.S012
        "s02" -> Test387State.S02
        "s021" -> Test387State.S021
        "s022" -> Test387State.S022
        "s1" -> Test387State.S1
        "s11" -> Test387State.S11
        "s111" -> Test387State.S111
        "s112" -> Test387State.S112
        "s12" -> Test387State.S12
        "s121" -> Test387State.S121
        "s122" -> Test387State.S122
        "s3" -> Test387State.S3
        "s4" -> Test387State.S4
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test387State): String = when (state) {
        is Test387State.Fail -> "fail"
        is Test387State.Pass -> "pass"
        is Test387State.S0 -> "s0"
        is Test387State.S01 -> "s01"
        is Test387State.S011 -> "s011"
        is Test387State.S012 -> "s012"
        is Test387State.S02 -> "s02"
        is Test387State.S021 -> "s021"
        is Test387State.S022 -> "s022"
        is Test387State.S1 -> "s1"
        is Test387State.S11 -> "s11"
        is Test387State.S111 -> "s111"
        is Test387State.S112 -> "s112"
        is Test387State.S12 -> "s12"
        is Test387State.S121 -> "s121"
        is Test387State.S122 -> "s122"
        is Test387State.S3 -> "s3"
        is Test387State.S4 -> "s4"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test387State): Boolean = when (state) {
        is Test387State.S0 -> false
        is Test387State.S01 -> false
        is Test387State.S02 -> false
        is Test387State.S1 -> false
        is Test387State.S11 -> false
        is Test387State.S12 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test387State): Int = when (state) {
        is Test387State.Fail -> 17
        is Test387State.Pass -> 16
        is Test387State.S0 -> 0
        is Test387State.S01 -> 1
        is Test387State.S011 -> 2
        is Test387State.S012 -> 3
        is Test387State.S02 -> 4
        is Test387State.S021 -> 5
        is Test387State.S022 -> 6
        is Test387State.S1 -> 7
        is Test387State.S11 -> 8
        is Test387State.S111 -> 9
        is Test387State.S112 -> 10
        is Test387State.S12 -> 11
        is Test387State.S121 -> 12
        is Test387State.S122 -> 13
        is Test387State.S3 -> 14
        is Test387State.S4 -> 15
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test387State,
        event: Test387Event
    ): TransitionResult<Test387State> = when (state) {
        is Test387State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test387State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test387State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s012 has no own event transitions)
        is Test387State.S012 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test387State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s021 has no own event transitions)
        is Test387State.S021 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s022 has no own event transitions)
        is Test387State.S022 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test387State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test387State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test387State.S111 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s112 has no own event transitions)
        is Test387State.S112 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s12 has no own event transitions)
        is Test387State.S12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s121 has no own event transitions)
        is Test387State.S121 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s122 has no own event transitions)
        is Test387State.S122 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test387State
    ): TransitionResult<Test387State> = when (state) {
        is Test387State.S3 -> processNullS3()
        is Test387State.S4 -> processNullS4()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS3(
    ): TransitionResult<Test387State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External((historyStore["s0HistShallow"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test387State.S011), Test387State.S3)
    }

    private fun processNullS4(
    ): TransitionResult<Test387State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External((historyStore["s1HistDeep"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test387State.S122), Test387State.S4)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test387Event
    ): TransitionResult<Test387State> = when {
        event is Test387Event.EnteringS011 -> TransitionResult.External(Test387State.S4, Test387State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test387State.Fail, Test387State.S0)
    }

    private fun processS1(
        event: Test387Event
    ): TransitionResult<Test387State> = when {
        event is Test387Event.EnteringS122 -> TransitionResult.External(Test387State.Pass, Test387State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test387State.Fail, Test387State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test387.scxml:7
    override fun onEntry(state: Test387State) {
        when (state) {
            is Test387State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test387State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test387State.S011 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s011")) return

            raiseInternal(Test387Event.EnteringS011)
            }
            is Test387State.S012 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s012")) return

            raiseInternal(Test387Event.EnteringS012)
            }
            is Test387State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test387State.S021 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s021")) return

            raiseInternal(Test387Event.EnteringS021)
            }
            is Test387State.S022 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s022")) return

            raiseInternal(Test387Event.EnteringS022)
            }
            is Test387State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test387State.S11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
            }
            is Test387State.S111 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s111")) return

            raiseInternal(Test387Event.EnteringS111)
            }
            is Test387State.S112 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s112")) return

            raiseInternal(Test387Event.EnteringS112)
            }
            is Test387State.S12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s12")) return
            }
            is Test387State.S121 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s121")) return

            raiseInternal(Test387Event.EnteringS121)
            }
            is Test387State.S122 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s122")) return

            raiseInternal(Test387Event.EnteringS122)
            }
            is Test387State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return


            scheduleSend("__send_0", 1000L, Test387Event.Timeout)
            }
            is Test387State.S4 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s4")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test387.scxml:7
    override fun onExit(state: Test387State) {
        when (state) {
            is Test387State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test387State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test387State.S0 -> {
                // W3C SCXML 3.11: Record deep history for s0HistDeep
                historyStore["s0HistDeep"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    isDescendantOf(st, Test387State.S0) && isAtomicState(st)
                }.toList()
                // W3C SCXML 3.11: Record shallow history for s0HistShallow
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["s0HistShallow"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "s0"
                }.toList()
                activeStateIds.remove("s0")
            }
            is Test387State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test387State.S011 -> {
                activeStateIds.remove("s011")
            }
            is Test387State.S012 -> {
                activeStateIds.remove("s012")
            }
            is Test387State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test387State.S021 -> {
                activeStateIds.remove("s021")
            }
            is Test387State.S022 -> {
                activeStateIds.remove("s022")
            }
            is Test387State.S1 -> {
                // W3C SCXML 3.11: Record deep history for s1HistDeep
                historyStore["s1HistDeep"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    isDescendantOf(st, Test387State.S1) && isAtomicState(st)
                }.toList()
                // W3C SCXML 3.11: Record shallow history for s1HistShallow
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["s1HistShallow"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "s1"
                }.toList()
                activeStateIds.remove("s1")
            }
            is Test387State.S11 -> {
                activeStateIds.remove("s11")
            }
            is Test387State.S111 -> {
                activeStateIds.remove("s111")
            }
            is Test387State.S112 -> {
                activeStateIds.remove("s112")
            }
            is Test387State.S12 -> {
                activeStateIds.remove("s12")
            }
            is Test387State.S121 -> {
                activeStateIds.remove("s121")
            }
            is Test387State.S122 -> {
                activeStateIds.remove("s122")
            }
            is Test387State.S3 -> {
                activeStateIds.remove("s3")
            }
            is Test387State.S4 -> {
                activeStateIds.remove("s4")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test387.scxml:7
    override fun executeTransitionActions(
        source: Test387State,
        event: Test387Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
