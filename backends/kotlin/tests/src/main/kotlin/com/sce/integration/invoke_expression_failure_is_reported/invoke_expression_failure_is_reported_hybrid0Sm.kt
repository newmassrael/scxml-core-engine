// SCE-GENERATED — DO NOT EDIT
// source-hash: 093b876a9ac3d5191526d3c37fa64f1e3d18e4b91f132c7e8e2c8dd8521dbdfb
// template-hash: de19bbe38b591cc01cdfdd92e882d57ccdf3a1af11ac8c67d61cfaa6882bea1d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: invoke_expression_failure_is_reported_hybrid0.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine

package com.sce.integration.invoke_expression_failure_is_reported

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeExpressionFailureIsReportedHybrid0State : State {
    data object Final : InvokeExpressionFailureIsReportedHybrid0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeExpressionFailureIsReportedHybrid0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class InvokeExpressionFailureIsReportedHybrid0StateMachine(
) : StateMachineEngine<InvokeExpressionFailureIsReportedHybrid0State, InvokeExpressionFailureIsReportedHybrid0Event>() {

    override val initialState: InvokeExpressionFailureIsReportedHybrid0State = InvokeExpressionFailureIsReportedHybrid0State.Final

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokeExpressionFailureIsReportedHybrid0State? = when (stateId) {
        "final" -> InvokeExpressionFailureIsReportedHybrid0State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeExpressionFailureIsReportedHybrid0State): String = when (state) {
        is InvokeExpressionFailureIsReportedHybrid0State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeExpressionFailureIsReportedHybrid0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeExpressionFailureIsReportedHybrid0State): Int = when (state) {
        is InvokeExpressionFailureIsReportedHybrid0State.Final -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeExpressionFailureIsReportedHybrid0Event? = when (name) {
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `InvokeExpressionFailureIsReportedHybrid0Event` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: InvokeExpressionFailureIsReportedHybrid0Event): String? = null




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokeExpressionFailureIsReportedHybrid0State,
        event: InvokeExpressionFailureIsReportedHybrid0Event
    ): TransitionResult<InvokeExpressionFailureIsReportedHybrid0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine
    override fun onEntry(state: InvokeExpressionFailureIsReportedHybrid0State, pathChild: InvokeExpressionFailureIsReportedHybrid0State?) {
        when (state) {
            is InvokeExpressionFailureIsReportedHybrid0State.Final -> {
                // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:3 :: final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine
    override fun onExit(state: InvokeExpressionFailureIsReportedHybrid0State) {
        when (state) {
            is InvokeExpressionFailureIsReportedHybrid0State.Final -> {
                // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:3 :: final :: _state_body
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine
    override fun executeTransitionActions(
        source: InvokeExpressionFailureIsReportedHybrid0State,
        event: InvokeExpressionFailureIsReportedHybrid0Event?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
