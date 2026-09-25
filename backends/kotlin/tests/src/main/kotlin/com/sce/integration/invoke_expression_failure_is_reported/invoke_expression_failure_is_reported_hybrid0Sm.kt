// SCE-GENERATED — DO NOT EDIT
// source-hash: 093b876a9ac3d5191526d3c37fa64f1e3d18e4b91f132c7e8e2c8dd8521dbdfb

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InvokeExpressionFailureIsReportedHybrid0State): Boolean = when (state) {
        is InvokeExpressionFailureIsReportedHybrid0State.Final -> true
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokeExpressionFailureIsReportedHybrid0State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokeExpressionFailureIsReportedHybrid0State, HistoryId>> =
            listOf(StateTarget(InvokeExpressionFailureIsReportedHybrid0State.Final))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokeExpressionFailureIsReportedHybrid0State? = when (stateId) {
        "final" -> InvokeExpressionFailureIsReportedHybrid0State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeExpressionFailureIsReportedHybrid0State): String = when (state) {
        is InvokeExpressionFailureIsReportedHybrid0State.Final -> "final"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: InvokeExpressionFailureIsReportedHybrid0State,
        event: InvokeExpressionFailureIsReportedHybrid0Event?
    ): EnabledTransition<InvokeExpressionFailureIsReportedHybrid0State, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine
    override fun onEntry(state: InvokeExpressionFailureIsReportedHybrid0State, isDefaultEntry: Boolean) {
        when (state) {
            is InvokeExpressionFailureIsReportedHybrid0State.Final -> {
                // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:3 :: final :: _state_body
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
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_expression_failure_is_reported_hybrid0.scxml:2 :: _machine
    override fun executeTransitionContent(source: InvokeExpressionFailureIsReportedHybrid0State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
