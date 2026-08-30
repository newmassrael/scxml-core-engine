// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Transition result types

package com.sce.runtime

/**
 * Result of processing an event in a given state.
 *
 * No lambdas or side effects — the engine calls onExit/onEntry/executeTransitionActions
 * in W3C-specified order based on the result type.
 *
 * §scxml-3.13: Microstep semantics.
 */
sealed interface TransitionResult<out S> {

    /**
     * WHICH transition this is, machine-wide, or [NO_TRANSITION] for [Ignored].
     *
     * ⚠ Load-bearing, and the defect it exists to close is not hypothetical.
     * Before this field the generated `executeTransitionActions` re-decided
     * which transition had been taken by RE-EVALUATING the same `cond` the
     * selection had already evaluated. For a pure guard that is invisible; for
     * a guard with a side effect it is wrong. Measured 2026-08-30 on
     * `++v == 2`: selection ran `++v`, read 2 and took the transition, then
     * the action dispatch ran `++v` again, read 3, and executed the OTHER
     * arm's content. The other backends never had this — C++, Rust, Go and
     * Python have all carried a transition index on their transition result
     * from the start, and Kotlin was the one that did not.
     *
     * ⚠⚠ MACHINE-WIDE, not the per-state `transition_index` the C++ template
     * switches on. C++ dispatches on `(source state, index within that
     * state's own list)`; Kotlin's dispatch table for a state is its
     * EFFECTIVE transitions — its own followed by every ancestor's — so a
     * child's transition 0 and its ancestor's transition 0 collide under one
     * `source`. `sce-build`'s `assign_machine_transition_ids` is the single
     * place the numbering is decided, and both halves of the generator read
     * it rather than counting for themselves.
     */
    val transitionIndex: Int

    /**
     * External transition: state changes from current to [target].
     *
     * §scxml-3.13: Exit source state(s), execute transition actions, enter target state(s).
     *
     * @param transitionSource The state where the transition is defined (for ancestor
     *        transitions). When set, the exit computation uses this as the effective source
     *        instead of the leaf state, ensuring correct LCCA computation.
     */
    data class External<S>(
        val target: S,
        val transitionSource: S? = null,
        override val transitionIndex: Int,
    ) : TransitionResult<S>

    /**
     * Internal transition: execute actions without changing state.
     *
     * §scxml-3.13: type="internal" — actions only, no exit/entry.
     * Used for targetless internal transitions.
     *
     * ⚠ A class rather than the `data object` it used to be, because it has to
     * carry [transitionIndex] like the other two. A targetless internal
     * transition is exactly the case whose whole observable effect is its
     * executable content, so it is the one that can least afford the dispatch
     * guessing which content to run.
     */
    data class Internal(override val transitionIndex: Int) : TransitionResult<Nothing>

    /**
     * Internal transition with a target state.
     *
     * §scxml-3.13: type="internal" where target is a proper descendant of the
     * transition's source state AND source is compound. Exits descendants of
     * [transitionSource] that are not the target, executes actions, enters target.
     * The [transitionSource] itself is NOT exited.
     */
    data class InternalToTarget<S>(
        val target: S,
        val transitionSource: S,
        override val transitionIndex: Int,
    ) : TransitionResult<S>

    /**
     * No matching transition for the event in this state.
     *
     * Event is silently ignored (§scxml-3.12: unhandled events are discarded).
     */
    data object Ignored : TransitionResult<Nothing> {
        override val transitionIndex: Int get() = NO_TRANSITION
    }

    companion object {
        /**
         * No transition was selected, so no action dispatch may match.
         *
         * ⚠ NEGATIVE on purpose. `0` is a real transition — the first one in
         * the machine — so a sentinel of 0 would make "nothing was selected"
         * indistinguishable from "the first transition was selected", and the
         * generated `when (transitionIndex)` would run that transition's
         * content for an ignored event.
         */
        const val NO_TRANSITION: Int = -1
    }
}
