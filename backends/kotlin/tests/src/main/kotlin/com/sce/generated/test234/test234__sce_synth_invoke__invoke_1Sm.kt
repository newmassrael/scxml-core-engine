// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: daab006ae7e0eecfbb4178e7ccbc8a9f062b3756932cf56951788a953474a874
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine

package com.sce.generated.test234

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test234SceSynthInvokeInvoke1State : State {
    data object Sub0 : Test234SceSynthInvokeInvoke1State
    data object SubFinal2 : Test234SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test234SceSynthInvokeInvoke1Event : Event {
    sealed interface Error : Test234SceSynthInvokeInvoke1Event {
        data object Execution : Error
    }
    data object Timeout : Test234SceSynthInvokeInvoke1Event
}
// --- State Machine (W3C SCXML) ---

class Test234SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test234SceSynthInvokeInvoke1State, Test234SceSynthInvokeInvoke1Event>() {

    override val initialState: Test234SceSynthInvokeInvoke1State = Test234SceSynthInvokeInvoke1State.Sub0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test234SceSynthInvokeInvoke1State? = when (stateId) {
        "sub0" -> Test234SceSynthInvokeInvoke1State.Sub0
        "subFinal2" -> Test234SceSynthInvokeInvoke1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test234SceSynthInvokeInvoke1State): String = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> "sub0"
        is Test234SceSynthInvokeInvoke1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test234SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test234SceSynthInvokeInvoke1State): Int = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> 0
        is Test234SceSynthInvokeInvoke1State.SubFinal2 -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test234SceSynthInvokeInvoke1State,
        event: Test234SceSynthInvokeInvoke1Event
    ): TransitionResult<Test234SceSynthInvokeInvoke1State> = when (state) {
        is Test234SceSynthInvokeInvoke1State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test234SceSynthInvokeInvoke1Event
    ): TransitionResult<Test234SceSynthInvokeInvoke1State> = when {
        event is Test234SceSynthInvokeInvoke1Event.Timeout -> TransitionResult.External(Test234SceSynthInvokeInvoke1State.SubFinal2, Test234SceSynthInvokeInvoke1State.Sub0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onEntry(state: Test234SceSynthInvokeInvoke1State, pathChild: Test234SceSynthInvokeInvoke1State?) {
        when (state) {
            is Test234SceSynthInvokeInvoke1State.Sub0 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:4 :: sub0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("__send_0", 2000L, Test234SceSynthInvokeInvoke1Event.Timeout)
            }
            is Test234SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:10 :: subFinal2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onExit(state: Test234SceSynthInvokeInvoke1State) {
        when (state) {
            is Test234SceSynthInvokeInvoke1State.Sub0 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:4 :: sub0 :: _state_body
                activeStateIds.remove("sub0")
            }
            is Test234SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:10 :: subFinal2 :: _state_body
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test234__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test234SceSynthInvokeInvoke1State,
        event: Test234SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
