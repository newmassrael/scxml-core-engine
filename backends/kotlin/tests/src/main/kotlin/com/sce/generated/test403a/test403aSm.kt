// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 32616a8c15423facd5b04f320c1acfc24557f2b58b0b9ca0229cb783903eb112
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/403/test403a.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test403a.scxml:11

package com.sce.generated.test403a

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test403aState : State {
    data object Fail : Test403aState
    data object Pass : Test403aState
    data object S0 : Test403aState
    data object S01 : Test403aState
    data object S02 : Test403aState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test403aEvent : Event {
    sealed interface Error : Test403aEvent {
        data object Execution : Error
    }
    data object Event1 : Test403aEvent
    data object Event2 : Test403aEvent
    data object Timeout : Test403aEvent
}
// --- State Machine (W3C SCXML) ---

class Test403aStateMachine(
) : StateMachineEngine<Test403aState, Test403aEvent>() {

    override val initialState: Test403aState = Test403aState.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test403aState): Test403aState? = when (state) {
        is Test403aState.S01 -> Test403aState.S0
        is Test403aState.S02 -> Test403aState.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test403aState): Test403aState = when (state) {
        is Test403aState.S0 -> Test403aState.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test403aState? = when (stateId) {
        "fail" -> Test403aState.Fail
        "pass" -> Test403aState.Pass
        "s0" -> Test403aState.S0
        "s01" -> Test403aState.S01
        "s02" -> Test403aState.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test403aState): String = when (state) {
        is Test403aState.Fail -> "fail"
        is Test403aState.Pass -> "pass"
        is Test403aState.S0 -> "s0"
        is Test403aState.S01 -> "s01"
        is Test403aState.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test403aState): Boolean = when (state) {
        is Test403aState.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test403aState): Int = when (state) {
        is Test403aState.Fail -> 4
        is Test403aState.Pass -> 3
        is Test403aState.S0 -> 0
        is Test403aState.S01 -> 1
        is Test403aState.S02 -> 2
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test403aState,
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when (state) {
        is Test403aState.S0 -> processS0(event)
        is Test403aState.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test403aState.S02 -> {
            val result = processS02(event)
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


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Timeout -> TransitionResult.External(Test403aState.Fail, Test403aState.S0)

        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.Fail, Test403aState.S0)

        event is Test403aEvent.Event2 -> TransitionResult.External(Test403aState.Pass, Test403aState.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.S02, Test403aState.S01)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test403aState.Fail, Test403aState.S01)
    }

    private fun processS02(
        event: Test403aEvent
    ): TransitionResult<Test403aState> = when {
        event is Test403aEvent.Event1 -> TransitionResult.External(Test403aState.Fail, Test403aState.S02)

        event is Test403aEvent.Event2 && false -> TransitionResult.External(Test403aState.Fail, Test403aState.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test403a.scxml:11
    override fun onEntry(state: Test403aState) {
        when (state) {
            is Test403aState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403aState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403aState.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test403aEvent.Timeout)
            }
            is Test403aState.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return

            raiseInternal(Test403aEvent.Event1)
            }
            is Test403aState.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return

            raiseInternal(Test403aEvent.Event2)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test403a.scxml:11
    override fun onExit(state: Test403aState) {
        when (state) {
            is Test403aState.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test403aState.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test403aState.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test403aState.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test403aState.S02 -> {
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test403a.scxml:11
    override fun executeTransitionActions(
        source: Test403aState,
        event: Test403aEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
