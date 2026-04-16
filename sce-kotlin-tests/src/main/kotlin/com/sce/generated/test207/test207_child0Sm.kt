
// GENERATED CODE — DO NOT EDIT
// Source: resources/207/test207_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test207

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test207Child0State : State {
    data object Sub0 : Test207Child0State
    data object SubFinal : Test207Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test207Child0Event : Event {
    data object ChildToParent : Test207Child0Event
    sealed interface Error : Test207Child0Event {
        data object Execution : Error
    }
    data object Event1 : Test207Child0Event
    data object Event2 : Test207Child0Event
    data object Fail : Test207Child0Event
    data object Pass : Test207Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test207Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test207Child0State, Test207Child0Event>(scriptEngine) {

    override val initialState: Test207Child0State = Test207Child0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test207Child0State? = when (stateId) {
        "sub0" -> Test207Child0State.Sub0
        "subFinal" -> Test207Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test207Child0State): String = when (state) {
        is Test207Child0State.Sub0 -> "sub0"
        is Test207Child0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test207Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test207Child0State): Int = when (state) {
        is Test207Child0State.Sub0 -> 0
        is Test207Child0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test207Child0Event? = when (name) {
        "childToParent" -> Test207Child0Event.ChildToParent
        "error.execution" -> Test207Child0Event.Error.Execution
        "event1" -> Test207Child0Event.Event1
        "event2" -> Test207Child0Event.Event2
        "fail" -> Test207Child0Event.Fail
        "pass" -> Test207Child0Event.Pass
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test207Child0Event): String? = when (event) {
        is Test207Child0Event.ChildToParent -> "childToParent"
        is Test207Child0Event.Error.Execution -> "error.execution"
        is Test207Child0Event.Event1 -> "event1"
        is Test207Child0Event.Event2 -> "event2"
        is Test207Child0Event.Fail -> "fail"
        is Test207Child0Event.Pass -> "pass"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test207Child0State,
        event: Test207Child0Event
    ): TransitionResult<Test207Child0State> = when (state) {
        is Test207Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test207Child0Event
    ): TransitionResult<Test207Child0State> = when {
        event is Test207Child0Event.Event1 -> TransitionResult.External(Test207Child0State.SubFinal, Test207Child0State.Sub0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test207Child0State.SubFinal, Test207Child0State.Sub0)
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test207Child0State) {
        when (state) {
            is Test207Child0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            scheduleSend("foo", 1000L, Test207Child0Event.Event1)


            scheduleSend("__send_2", 1500L, Test207Child0Event.Event2)


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test207Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test207Child0State) {
        when (state) {
            is Test207Child0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test207Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test207Child0State,
        event: Test207Child0Event?
    ) {
        when (source) {
        is Test207Child0State.Sub0 -> when {
            event is Test207Child0Event.Event1 -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("pass", "")
            }
            event != null -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("fail", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
