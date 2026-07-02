// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Transition history record

package com.sce.runtime

/**
 * Immutable record of a state transition for debugging and logging.
 *
 * Emitted via `SharedFlow<TransitionRecord>` (no conflation) so that
 * every transition is observed, even rapid sequential ones.
 *
 * @param source State before the transition
 * @param event Event that triggered the transition
 * @param target State after the transition
 * @param timestamp Monotonic time in milliseconds (System.nanoTime() / 1_000_000)
 */
data class TransitionRecord<S, E>(
    val source: S,
    val event: E,
    val target: S,
    val timestamp: Long
)
