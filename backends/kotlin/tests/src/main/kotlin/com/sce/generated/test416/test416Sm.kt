// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 123759fa1515134527b83cfd094acff4a38d0e67d776745e7939fe5a5955e20a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/416/test416.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test416.scxml:5 :: _machine

package com.sce.generated.test416

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test416State : State {
    data object Fail : Test416State
    data object Pass : Test416State
    data object S1 : Test416State
    data object S11 : Test416State
    data object S111 : Test416State
    data object S11final : Test416State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test416Event : Event {
    sealed interface Done : Test416Event {
        sealed interface State : Done {
            data object S11 : State
        }
    }
    sealed interface Error : Test416Event {
        data object Execution : Error
    }
    data object Timeout : Test416Event
}
// --- State Machine (W3C SCXML) ---

class Test416StateMachine(
) : StateMachineEngine<Test416State, Test416Event>() {

    override val initialState: Test416State = Test416State.S111

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test416State): Test416State? = when (state) {
        is Test416State.S11 -> Test416State.S1
        is Test416State.S111 -> Test416State.S11
        is Test416State.S11final -> Test416State.S11
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test416State): Test416State = when (state) {
        is Test416State.S1 -> Test416State.S111
        is Test416State.S11 -> Test416State.S111
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test416State? = when (stateId) {
        "fail" -> Test416State.Fail
        "pass" -> Test416State.Pass
        "s1" -> Test416State.S1
        "s11" -> Test416State.S11
        "s111" -> Test416State.S111
        "s11final" -> Test416State.S11final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test416State): String = when (state) {
        is Test416State.Fail -> "fail"
        is Test416State.Pass -> "pass"
        is Test416State.S1 -> "s1"
        is Test416State.S11 -> "s11"
        is Test416State.S111 -> "s111"
        is Test416State.S11final -> "s11final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test416State): Boolean = when (state) {
        is Test416State.S1 -> false
        is Test416State.S11 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test416State): Int = when (state) {
        is Test416State.Fail -> 5
        is Test416State.Pass -> 4
        is Test416State.S1 -> 0
        is Test416State.S11 -> 1
        is Test416State.S111 -> 2
        is Test416State.S11final -> 3
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test416State,
        event: Test416Event
    ): TransitionResult<Test416State> = when (state) {
        is Test416State.S1 -> processS1(event)
        is Test416State.S11 -> {
            val result = processS11(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS1(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s111 has no own event transitions)
        is Test416State.S111 -> {
            val anc1 = processS11(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s11final has no own event transitions)
        is Test416State.S11final -> {
            val anc1 = processS11(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS1(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test416State
    ): TransitionResult<Test416State> = when (state) {
        is Test416State.S111 -> processNullS111()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS111(
    ): TransitionResult<Test416State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test416State.S11final, Test416State.S111)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test416Event
    ): TransitionResult<Test416State> = when {
        event is Test416Event.Timeout -> TransitionResult.External(Test416State.Fail, Test416State.S1)

        else -> TransitionResult.Ignored
    }

    private fun processS11(
        event: Test416Event
    ): TransitionResult<Test416State> = when {
        event is Test416Event.Done.State.S11 -> TransitionResult.External(Test416State.Pass, Test416State.S11)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun onEntry(state: Test416State, pathChild: Test416State?) {
        when (state) {
            is Test416State.Fail -> {
                // SCE-MAP: test416.scxml:24 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test416State.Pass -> {
                // SCE-MAP: test416.scxml:23 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test416State.S1 -> {
                // SCE-MAP: test416.scxml:7 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_0", 1000L, Test416Event.Timeout)
            }
            is Test416State.S11 -> {
                // SCE-MAP: test416.scxml:13 :: s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
            }
            is Test416State.S111 -> {
                // SCE-MAP: test416.scxml:15 :: s111 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s111")) return
            }
            is Test416State.S11final -> {
                // SCE-MAP: test416.scxml:18 :: s11final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11final")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test416Event.Done.State.S11, EventMetadata.platform())
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun onExit(state: Test416State) {
        when (state) {
            is Test416State.Fail -> {
                // SCE-MAP: test416.scxml:24 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test416State.Pass -> {
                // SCE-MAP: test416.scxml:23 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test416State.S1 -> {
                // SCE-MAP: test416.scxml:7 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test416State.S11 -> {
                // SCE-MAP: test416.scxml:13 :: s11 :: _state_body
                activeStateIds.remove("s11")
            }
            is Test416State.S111 -> {
                // SCE-MAP: test416.scxml:15 :: s111 :: _state_body
                activeStateIds.remove("s111")
            }
            is Test416State.S11final -> {
                // SCE-MAP: test416.scxml:18 :: s11final :: _state_body
                activeStateIds.remove("s11final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test416.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test416State,
        event: Test416Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
