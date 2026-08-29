// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared ECMA-262 table, read once for every suite on this backend that
// asks it something.
//
// Two suites ask it two different questions — `EcmaScriptSemanticsTest` asks
// what each engine answers for the AUTHOR'S ECMAScript, `LoweredEcma262Test`
// asks what this backend's Lua answers for what the frontend EMITTED — and
// the row, the answer shapes and the number comparison are the same in both.
// A copy in each would be the per-backend copy the table's own header refuses
// one layer down: it drifts toward whichever suite edits it, and the two
// would stop being comparable exactly when a disagreement between them is
// the interesting result.

package com.sce.ecmascript

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.double
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.math.abs
import kotlin.test.assertTrue

/**
 * Paths, relative to the repository root — the suites' `workingDir`, which is
 * also how the C++, Rust, Go and Python readers name the same files.
 */
internal const val SHARED_TABLE_PATH = "tests/ecmascript/ecma262_semantics.json"

/**
 * A floor, not an equality: adding a case must not have to touch this number,
 * but a table that stopped being read must not pass either. The same floor
 * every other reader of this table applies, and it is the arity check that
 * keeps an empty sweep from satisfying every assertion above it.
 */
internal const val SHARED_TABLE_FLOOR = 55

/** What identifies one case, and one declared divergence against it. */
internal data class Key(val source: String, val clause: String)

/** One row of the shared table. */
internal class Ecma262Case(
    val setup: String,
    val source: String,
    val asCondition: Boolean,
    val clause: String,
    val expected: Answer,
) {
    val key: Key get() = Key(source, clause)
}

/** The single answer a row names: exactly one of these shapes. */
internal sealed interface Answer {
    fun describe(): String

    class Bool(val value: Boolean) : Answer {
        override fun describe() = value.toString()
    }

    class Num(val value: Double) : Answer {
        override fun describe() = value.toString()
    }

    class Text(val value: String) : Answer {
        override fun describe() = "\"$value\""
    }

    object Empty : Answer {
        override fun describe() = "null or undefined"
    }
}

/**
 * The shared table, in file order.
 *
 * File order matters to more than one caller: the emission beside this table
 * is generated case by case in the same order, and the pairing is asserted
 * rather than assumed by whoever reads both.
 */
internal fun loadSharedTable(): List<Ecma262Case> {
    val table = File(SHARED_TABLE_PATH)
    assertTrue(
        table.isFile,
        "the shared ECMA-262 table is missing at ${table.absolutePath}; " +
            "this test measures nothing without it",
    )
    val root = Json.parseToJsonElement(table.readText()).jsonObject
    val cases = root.getValue("cases").jsonArray.map { entry ->
        val obj = entry.jsonObject
        Ecma262Case(
            setup = obj["setup"]?.jsonPrimitive?.contentOrNull.orEmpty(),
            source = obj.getValue("source").jsonPrimitive.content,
            asCondition = obj.getValue("form").jsonPrimitive.content == "condition",
            clause = obj.getValue("clause").jsonPrimitive.content,
            expected = parseAnswer(obj.getValue("expect").jsonObject, obj),
        )
    }
    assertTrue(
        cases.size >= SHARED_TABLE_FLOOR,
        "the shared ECMA-262 table produced only ${cases.size} case(s), " +
            "so this is not measuring the corpus it claims to",
    )
    return cases
}

private fun parseAnswer(expect: JsonObject, row: JsonObject): Answer {
    expect["bool"]?.let { return Answer.Bool(it.jsonPrimitive.boolean) }
    expect["number"]?.let { return Answer.Num(it.jsonPrimitive.double) }
    expect["string"]?.let { return Answer.Text(it.jsonPrimitive.content) }
    // `empty` carries no value of its own — its presence IS the answer.
    expect["empty"]?.let { return Answer.Empty }
    // A case whose expectation cannot be read is not a case that passes:
    // reading it as "no answer" would let a typo in a key name retire a case
    // silently, which is the failure mode the shared table exists to remove.
    throw IllegalStateException(
        "case ${row.getValue("source").jsonPrimitive.content} names no expected answer",
    )
}

/**
 * An engine may hold a whole number as an integer or as a double, and
 * ECMA-262 has one Number type — so both spellings answer a `number` case.
 * The same rule the C++, Go, Rust and Python readers apply, for the same
 * reason.
 */
internal fun answerMatches(actual: Any?, expected: Answer): Boolean = when (expected) {
    is Answer.Bool -> actual is Boolean && actual == expected.value
    is Answer.Num -> actual is Number && abs(actual.toDouble() - expected.value) < 1e-9
    is Answer.Text -> actual is String && actual == expected.value
    Answer.Empty -> actual == null || isUndefined(actual)
}

/**
 * Rhino hands back its own singleton for `undefined` rather than a Kotlin
 * `null`, and the table treats null and undefined as one answer because
 * ECMAScript's `==` equates them and SCXML's datamodel cannot tell an absent
 * property from a null one.
 */
private fun isUndefined(value: Any): Boolean =
    value.javaClass.name == "org.mozilla.javascript.Undefined" || value.toString() == "undefined"

internal fun describeValue(value: Any?): String = when (value) {
    null -> "null"
    is String -> "\"$value\""
    else -> "$value (${value.javaClass.simpleName})"
}

/** JSON-quoted, so a failure prints entries that can be pasted as-is. */
internal fun jsonQuoted(value: String): String =
    "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"") + "\""
