// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 10c5bb56d60f6d5bc4121611a1230324eaf61d1a5524b71d52c6010f279d5ffd
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/412/test412.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test412.scxml:6 :: _machine

package com.sce.generated.test412

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test412State : State {
    data object Fail : Test412State
    data object Pass : Test412State
    data object S0 : Test412State
    data object S01 : Test412State
    data object S011 : Test412State
    data object S02 : Test412State
    data object S03 : Test412State
    data object S04 : Test412State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test412Event : Event {
    sealed interface Error : Test412Event {
        data object Execution : Error
    }
    data object Event1 : Test412Event
    data object Event2 : Test412Event
    data object Event3 : Test412Event
    data object Timeout : Test412Event
}
// --- State Machine (W3C SCXML) ---

class Test412StateMachine(
) : StateMachineEngine<Test412State, Test412Event>() {

    override val initialState: Test412State = Test412State.S011

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test412State): Test412State? = when (state) {
        is Test412State.S01 -> Test412State.S0
        is Test412State.S011 -> Test412State.S01
        is Test412State.S02 -> Test412State.S0
        is Test412State.S03 -> Test412State.S0
        is Test412State.S04 -> Test412State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test412State): Test412State = when (state) {
        is Test412State.S0 -> Test412State.S011
        is Test412State.S01 -> Test412State.S011
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test412State? = when (stateId) {
        "fail" -> Test412State.Fail
        "pass" -> Test412State.Pass
        "s0" -> Test412State.S0
        "s01" -> Test412State.S01
        "s011" -> Test412State.S011
        "s02" -> Test412State.S02
        "s03" -> Test412State.S03
        "s04" -> Test412State.S04
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test412State): String = when (state) {
        is Test412State.Fail -> "fail"
        is Test412State.Pass -> "pass"
        is Test412State.S0 -> "s0"
        is Test412State.S01 -> "s01"
        is Test412State.S011 -> "s011"
        is Test412State.S02 -> "s02"
        is Test412State.S03 -> "s03"
        is Test412State.S04 -> "s04"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test412State): Boolean = when (state) {
        is Test412State.S0 -> false
        is Test412State.S01 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test412State): Int = when (state) {
        is Test412State.Fail -> 7
        is Test412State.Pass -> 6
        is Test412State.S0 -> 0
        is Test412State.S01 -> 1
        is Test412State.S011 -> 2
        is Test412State.S02 -> 3
        is Test412State.S03 -> 4
        is Test412State.S04 -> 5
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test412State,
        event: Test412Event
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test412State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test412State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test412State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test412State.S04 -> {
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
        state: Test412State
    ): TransitionResult<Test412State> = when (state) {
        is Test412State.S011 -> processNullS011()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS011(
    ): TransitionResult<Test412State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test412State.S02, Test412State.S011)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Timeout -> TransitionResult.External(Test412State.Fail, Test412State.S0)

        event is Test412Event.Event1 -> TransitionResult.External(Test412State.Fail, Test412State.S0)

        event is Test412Event.Event2 -> TransitionResult.External(Test412State.Pass, Test412State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event1 -> TransitionResult.External(Test412State.S03, Test412State.S02)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S02)
    }

    private fun processS03(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event2 -> TransitionResult.External(Test412State.S04, Test412State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S03)
    }

    private fun processS04(
        event: Test412Event
    ): TransitionResult<Test412State> = when {
        event is Test412Event.Event3 -> TransitionResult.External(Test412State.Pass, Test412State.S04)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test412State.Fail, Test412State.S04)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun onEntry(state: Test412State, pathChild: Test412State?) {
        when (state) {
            is Test412State.Fail -> {
                // SCE-MAP: test412.scxml:54 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.Pass -> {
                // SCE-MAP: test412.scxml:53 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.S0 -> {
                // SCE-MAP: test412.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test412Event.Timeout)
            }
            is Test412State.S01 -> {
                // SCE-MAP: test412.scxml:18 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return

            raiseInternal(Test412Event.Event1)
                // W3C SCXML 3.3.2: Execute initial transition content

            raiseInternal(Test412Event.Event2)
            }
            is Test412State.S011 -> {
                // SCE-MAP: test412.scxml:28 :: s011 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s011")) return

            raiseInternal(Test412Event.Event3)
            }
            is Test412State.S02 -> {
                // SCE-MAP: test412.scxml:36 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test412State.S03 -> {
                // SCE-MAP: test412.scxml:41 :: s03 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test412State.S04 -> {
                // SCE-MAP: test412.scxml:46 :: s04 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s04")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun onExit(state: Test412State) {
        when (state) {
            is Test412State.Fail -> {
                // SCE-MAP: test412.scxml:54 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test412State.Pass -> {
                // SCE-MAP: test412.scxml:53 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test412State.S0 -> {
                // SCE-MAP: test412.scxml:9 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test412State.S01 -> {
                // SCE-MAP: test412.scxml:18 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test412State.S011 -> {
                // SCE-MAP: test412.scxml:28 :: s011 :: _state_body
                activeStateIds.remove("s011")
            }
            is Test412State.S02 -> {
                // SCE-MAP: test412.scxml:36 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
            is Test412State.S03 -> {
                // SCE-MAP: test412.scxml:41 :: s03 :: _state_body
                activeStateIds.remove("s03")
            }
            is Test412State.S04 -> {
                // SCE-MAP: test412.scxml:46 :: s04 :: _state_body
                activeStateIds.remove("s04")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test412State,
        event: Test412Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
