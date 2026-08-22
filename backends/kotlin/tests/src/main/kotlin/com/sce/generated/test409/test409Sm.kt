// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/409/test409.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test409.scxml:7 :: _machine

package com.sce.generated.test409

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test409State : State {
    data object Fail : Test409State
    data object Pass : Test409State
    data object S0 : Test409State
    data object S01 : Test409State
    data object S011 : Test409State
    data object S02 : Test409State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test409Event : Event {
    sealed interface Error : Test409Event {
        data object Execution : Error
    }
    data object Event1 : Test409Event
    data object Timeout : Test409Event
}
// --- State Machine (W3C SCXML) ---

class Test409StateMachine(
) : StateMachineEngine<Test409State, Test409Event>() {

    override val initialState: Test409State = Test409State.S011

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test409State): Test409State? = when (state) {
        is Test409State.S01 -> Test409State.S0
        is Test409State.S011 -> Test409State.S01
        is Test409State.S02 -> Test409State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test409State): Test409State = when (state) {
        is Test409State.S0 -> Test409State.S011
        is Test409State.S01 -> Test409State.S011
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test409State? = when (stateId) {
        "fail" -> Test409State.Fail
        "pass" -> Test409State.Pass
        "s0" -> Test409State.S0
        "s01" -> Test409State.S01
        "s011" -> Test409State.S011
        "s02" -> Test409State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test409State): String = when (state) {
        is Test409State.Fail -> "fail"
        is Test409State.Pass -> "pass"
        is Test409State.S0 -> "s0"
        is Test409State.S01 -> "s01"
        is Test409State.S011 -> "s011"
        is Test409State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test409State): Boolean = when (state) {
        is Test409State.S0 -> false
        is Test409State.S01 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test409State): Int = when (state) {
        is Test409State.Fail -> 5
        is Test409State.Pass -> 4
        is Test409State.S0 -> 0
        is Test409State.S01 -> 1
        is Test409State.S011 -> 2
        is Test409State.S02 -> 3
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test409State,
        event: Test409Event
    ): TransitionResult<Test409State> = when (state) {
        is Test409State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test409State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test409State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test409State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test409State
    ): TransitionResult<Test409State> = when (state) {
        is Test409State.S011 -> processNullS011()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS011(
    ): TransitionResult<Test409State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test409State.S02, Test409State.S011)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test409Event
    ): TransitionResult<Test409State> = when {
        event is Test409Event.Timeout -> TransitionResult.External(Test409State.Pass, Test409State.S0)

        event is Test409Event.Event1 -> TransitionResult.External(Test409State.Fail, Test409State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun onEntry(state: Test409State, pathChild: Test409State?) {
        when (state) {
            is Test409State.Fail -> {
                // SCE-MAP: test409.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test409State.Pass -> {
                // SCE-MAP: test409.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test409State.S0 -> {
                // SCE-MAP: test409.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test409Event.Timeout)
            }
            is Test409State.S01 -> {
                // SCE-MAP: test409.scxml:18 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test409State.S011 -> {
                // SCE-MAP: test409.scxml:25 :: s011 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s011")) return
            }
            is Test409State.S02 -> {
                // SCE-MAP: test409.scxml:30 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun onExit(state: Test409State) {
        when (state) {
            is Test409State.Fail -> {
                // SCE-MAP: test409.scxml:35 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test409State.Pass -> {
                // SCE-MAP: test409.scxml:34 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test409State.S0 -> {
                // SCE-MAP: test409.scxml:10 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test409State.S01 -> {
                // SCE-MAP: test409.scxml:18 :: s01 :: _state_body
                activeStateIds.remove("s01")


            if (isStateActive("s011")) {

            raiseInternal(Test409Event.Event1)
            }
            }
            is Test409State.S011 -> {
                // SCE-MAP: test409.scxml:25 :: s011 :: _state_body
                activeStateIds.remove("s011")
            }
            is Test409State.S02 -> {
                // SCE-MAP: test409.scxml:30 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test409.scxml:7 :: _machine
    override fun executeTransitionActions(
        source: Test409State,
        event: Test409Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
