// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Runtime — Transition result types

package com.sce.runtime

/**
 * Result of processing an event in a given state.
 *
 * No lambdas or side effects — the engine calls onExit/onEntry/executeTransitionActions
 * in W3C-specified order based on the result type.
 *
 * W3C SCXML 3.13: Microstep semantics.
 */
sealed interface TransitionResult<out S> {

    /**
     * External transition: state changes from current to [target].
     *
     * W3C SCXML 3.13: Exit source state(s), execute transition actions, enter target state(s).
     */
    data class External<S>(val target: S) : TransitionResult<S>

    /**
     * Internal transition: execute actions without changing state.
     *
     * W3C SCXML 3.13: type="internal" — actions only, no exit/entry.
     */
    data object Internal : TransitionResult<Nothing>

    /**
     * No matching transition for the event in this state.
     *
     * Event is silently ignored (W3C SCXML 3.12: unhandled events are discarded).
     */
    data object Ignored : TransitionResult<Nothing>
}
