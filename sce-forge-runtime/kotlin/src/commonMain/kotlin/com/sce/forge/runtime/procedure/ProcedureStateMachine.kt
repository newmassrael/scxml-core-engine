// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
// SCE Forge: Base class for event-driven (Level 2) procedure state machines.
//
// Generated code extends this class and implements the abstract methods.
// The event loop lives here — generated code provides only the policy.

package com.sce.forge.runtime.procedure

/**
 * Abstract base class for Level 2 (event-driven) procedure state machines.
 *
 * Type parameters:
 *   S — State enum type (generated per procedure)
 *   E — Event enum type (generated per procedure)
 *
 * Generated code implements the abstract methods that define the state
 * machine's behavior. This class provides:
 *   - Service handler management
 *   - Event-driven execution loop (runToCompletion)
 *   - Done data and pending event data storage
 */
abstract class ProcedureStateMachine<S : Enum<S>, E : Enum<E>> {

    /** Service handler for <send sce:service> actions. */
    var serviceHandler: ProcedureServiceHandler? = null
        private set

    /** Done data collected from <donedata> in final states. */
    protected val doneData: MutableMap<String, String> = mutableMapOf()

    /** W3C SCXML 5.10: _event.data binding — populated by entry actions. */
    protected var pendingEventData: String = ""

    /** Set the service handler for <send sce:service> actions. */
    fun setServiceHandler(handler: ProcedureServiceHandler) {
        serviceHandler = handler
    }

    /**
     * Run the procedure to completion (blocking).
     *
     * Drives the state machine from the initial state through service sends
     * until a <final> state is reached or no transition is possible.
     */
    fun runToCompletion(): ProcedureRunResult {
        var current = initialState()
        var event = noneEvent

        val initial = executeEntryActions(current)
        event = initial.first
        if (initial.second.isNotEmpty()) pendingEventData = initial.second

        for (i in 0 until MAX_ITERATIONS) {
            if (isFinal(current)) break
            val transition = processTransition(current, event) ?: break
            if (transition.third) executeTransitionActions(current, transition.second)
            current = transition.first
            event = noneEvent
            val entry = executeEntryActions(current)
            event = entry.first
            if (entry.second.isNotEmpty()) pendingEventData = entry.second
        }

        val completed = isFinal(current)
        return ProcedureRunResult(
            completed = completed,
            finalState = if (completed) finalStateName(current) else "",
            doneData = if (completed) doneData.toMap() else emptyMap()
        )
    }

    companion object {
        /** Safety limit for the event loop — prevents infinite loops from misconfigured procedures. */
        const val MAX_ITERATIONS = 1000
    }

    // ── Abstract policy methods (implemented by generated code) ──

    /** The Event enum value representing "no event". */
    protected abstract val noneEvent: E

    /** Initial state of the procedure. */
    protected abstract fun initialState(): S

    /** Whether the given state is a <final> state. */
    protected abstract fun isFinal(state: S): Boolean

    /** SCXML id of the final state (e.g., "done", "error"). */
    protected abstract fun finalStateName(state: S): String

    /** Execute entry actions for a state; returns (event, eventData). */
    protected abstract fun executeEntryActions(state: S): Pair<E, String>

    /** Process a transition; returns (nextState, transitionIndex, hasAssigns) or null. */
    protected abstract fun processTransition(state: S, event: E): Triple<S, Int, Boolean>?

    /** Execute <assign> actions for a transition. */
    protected abstract fun executeTransitionActions(source: S, trIndex: Int)
}
