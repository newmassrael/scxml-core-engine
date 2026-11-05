// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Runtime — the transition a run took, as its observer is told it

package com.sce.runtime

/**
 * One transition the engine took, as its row of the emitted transition table
 * names it (docs/adr/0089, docs/SCE_TRANSITION_TABLE.md).
 *
 * [source] is the machine's own state type, the type the table's `from` holds,
 * and [index] is the table's `index` type, so a pair compares with a row
 * (`row.from to row.index`) without a conversion.
 *
 * @param source The state the transition is written in: the `from` of its row
 * @param index Its position among [source]'s own transitions in document order:
 *   the `index` of its row
 */
data class TransitionTaken<S>(
    val source: S,
    val index: Int,
)
