// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/192/test192.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test192.scxml:8 :: _machine

package com.sce.generated.test192

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test192State : State {
    data object Fail : Test192State
    data object Pass : Test192State
    data object S0 : Test192State
    data object S01 : Test192State
    data object S02 : Test192State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test192Event : Event {
    sealed interface Cancel : Test192Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test192Event
    sealed interface Done : Test192Event {
        data object Invoke : Done
    }
    sealed interface Error : Test192Event {
        data object Execution : Error
    }
    data object EventReceived : Test192Event
    data object ParentToChild : Test192Event
    data object Timeout : Test192Event
}
// --- State Machine (W3C SCXML) ---

class Test192StateMachine(
) : StateMachineEngine<Test192State, Test192Event>() {

    override val initialState: Test192State = Test192State.S01

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test192State): Test192State? = when (state) {
        is Test192State.S01 -> Test192State.S0
        is Test192State.S02 -> Test192State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test192State): Test192State = when (state) {
        is Test192State.S0 -> Test192State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test192State? = when (stateId) {
        "fail" -> Test192State.Fail
        "pass" -> Test192State.Pass
        "s0" -> Test192State.S0
        "s01" -> Test192State.S01
        "s02" -> Test192State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test192State): String = when (state) {
        is Test192State.Fail -> "fail"
        is Test192State.Pass -> "pass"
        is Test192State.S0 -> "s0"
        is Test192State.S01 -> "s01"
        is Test192State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test192State): Boolean = when (state) {
        is Test192State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test192State): Int = when (state) {
        is Test192State.Fail -> 4
        is Test192State.Pass -> 3
        is Test192State.S0 -> 0
        is Test192State.S01 -> 1
        is Test192State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test192Event? = when (name) {
        "cancel.invoke" -> Test192Event.Cancel.Invoke
        "childToParent" -> Test192Event.ChildToParent
        "done.invoke" -> Test192Event.Done.Invoke
        "error.execution" -> Test192Event.Error.Execution
        "eventReceived" -> Test192Event.EventReceived
        "parentToChild" -> Test192Event.ParentToChild
        "timeout" -> Test192Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test192Event): String? = when (event) {
        is Test192Event.Cancel.Invoke -> "cancel.invoke"
        is Test192Event.ChildToParent -> "childToParent"
        is Test192Event.Done.Invoke -> "done.invoke"
        is Test192Event.Error.Execution -> "error.execution"
        is Test192Event.EventReceived -> "eventReceived"
        is Test192Event.ParentToChild -> "parentToChild"
        is Test192Event.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test192State,
        event: Test192Event
    ): TransitionResult<Test192State> = when (state) {
        is Test192State.S0 -> processS0(event)
        is Test192State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test192State.S02 -> {
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
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.Timeout -> TransitionResult.External(Test192State.Fail, Test192State.S0)

        event is Test192Event.Done.Invoke -> TransitionResult.External(Test192State.Fail, Test192State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.ChildToParent -> TransitionResult.External(Test192State.S02, Test192State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test192Event
    ): TransitionResult<Test192State> = when {
        event is Test192Event.EventReceived -> TransitionResult.External(Test192State.Pass, Test192State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test192.scxml:8 :: _machine
    override fun onEntry(state: Test192State) {
        when (state) {
            is Test192State.Fail -> {
                // SCE-MAP: test192.scxml:56 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test192State.Pass -> {
                // SCE-MAP: test192.scxml:55 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test192State.S0 -> {
                // SCE-MAP: test192.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test192Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.invokedChild"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test192SceSynthInvokeInvokedChildStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("invokedChild", childSM, false, Test192Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test192State.S01 -> {
                // SCE-MAP: test192.scxml:43 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test192State.S02 -> {
                // SCE-MAP: test192.scxml:49 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test192.scxml:8 :: _machine
    override fun onExit(state: Test192State) {
        when (state) {
            is Test192State.Fail -> {
                // SCE-MAP: test192.scxml:56 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test192State.Pass -> {
                // SCE-MAP: test192.scxml:55 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test192State.S0 -> {
                // SCE-MAP: test192.scxml:10 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("invokedChild")
                activeStateIds.remove("s0")
            }
            is Test192State.S01 -> {
                // SCE-MAP: test192.scxml:43 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test192State.S02 -> {
                // SCE-MAP: test192.scxml:49 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test192.scxml:8 :: _machine
    override fun executeTransitionActions(
        source: Test192State,
        event: Test192Event?
    ) {
        when (source) {
        is Test192State.S01 -> when {
            event is Test192Event.ChildToParent -> {
                // SCE-MAP: test192.scxml:44 :: s01 :: _transition_0


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("invokedChild", "parentToChild")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
