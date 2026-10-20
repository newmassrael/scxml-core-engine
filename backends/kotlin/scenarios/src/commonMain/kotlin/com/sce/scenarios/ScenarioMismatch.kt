// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.scenarios

import kotlinx.serialization.json.JsonElement

/**
 * A machine that did not hold what a scenario says it holds, or a scenario its
 * driver cannot judge: the one way [replay] reports a failure.
 *
 * The Rust twin panics with the step and what differed; this carries the same
 * as fields, so a caller outside a test framework can read which step failed and
 * on what, and a test framework reports it as the failed assertion it is (it is
 * an [AssertionError]).
 *
 * [expected] and [actual] are the two sides of a comparison, both `null` when
 * the failure is not one (a step that states what a driver does not record). An
 * [actual] of `null` beside an [expected] is a value the machine does not hold
 * at all, such as a variable its saved state does not carry.
 */
class ScenarioMismatch(
    /** The step that failed, counted from 0; `null` when the scenario as a whole judges nothing. */
    val step: Int?,
    /** The step's `note`, empty when it has none. */
    val note: String,
    /** What the step held the machine to, in words: "the current state", "variable `count`". */
    val what: String,
    /** The scenario's answer. */
    val expected: JsonElement? = null,
    /** What the machine held. */
    val actual: JsonElement? = null,
) : AssertionError(describe(step, note, what, expected, actual))

/**
 * The message of a [ScenarioMismatch]: `step 3 (note): the current state is
 * "idle", not "busy"`, the shape the Rust twin's panic takes.
 */
private fun describe(step: Int?, note: String, what: String, expected: JsonElement?, actual: JsonElement?): String {
    val where = if (step == null) "" else "step $step ($note): "
    if (expected == null && actual == null) {
        return "$where$what"
    }
    return "$where$what is ${actual ?: "absent"}, not $expected"
}
