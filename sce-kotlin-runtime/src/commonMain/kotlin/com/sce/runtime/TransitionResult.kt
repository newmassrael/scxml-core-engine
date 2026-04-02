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
     *
     * @param transitionSource The state where the transition is defined (for ancestor
     *        transitions). When set, the exit computation uses this as the effective source
     *        instead of the leaf state, ensuring correct LCCA computation.
     */
    data class External<S>(val target: S, val transitionSource: S? = null) : TransitionResult<S>

    /**
     * Internal transition: execute actions without changing state.
     *
     * W3C SCXML 3.13: type="internal" — actions only, no exit/entry.
     * Used for targetless internal transitions.
     */
    data object Internal : TransitionResult<Nothing>

    /**
     * Internal transition with a target state.
     *
     * W3C SCXML 3.13: type="internal" where target is a proper descendant of the
     * transition's source state AND source is compound. Exits descendants of
     * [transitionSource] that are not the target, executes actions, enters target.
     * The [transitionSource] itself is NOT exited.
     */
    data class InternalToTarget<S>(val target: S, val transitionSource: S) : TransitionResult<S>

    /**
     * No matching transition for the event in this state.
     *
     * Event is silently ignored (W3C SCXML 3.12: unhandled events are discarded).
     */
    data object Ignored : TransitionResult<Nothing>
}
