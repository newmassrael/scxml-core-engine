// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Kotlin conformance gate runs every route into every engine, or is red.
//
// `scripts/gates/w3c-kotlin.sh` names the (engine, artifact language) pairs it
// runs. That array is a POPULATION claim — "these are the ways a generated
// machine can reach an engine in this backend" — and until this file existed
// nothing checked it. The array said `(rhino quickjs)` for months while a
// third engine shipped, and the comment above it grew into a paragraph of
// prose explaining the omission. Prose is not a lane: the omission survived
// every push because the only thing that could have contradicted it was the
// same array.
//
// So the population is DERIVED here and the array is compared against it. The
// engines come from `W3CTestBase.KNOWN_ENGINES`, which is what the suite will
// actually build; the languages each engine can be handed come from
// `ScxmlScriptEngine.acceptsLanguage`, which is what the engine itself answers
// when a machine hands it a `ScriptSource`. Neither is a list this file keeps.
//
// ⚠ The direction that matters is the MISSING row, not the extra one. A row
// naming a pair nothing supports fails loudly the moment the gate runs it —
// the engine refuses the language. A pair the array omits fails nowhere,
// which is the shape every entry in this repository's debt ledger has.

package com.sce.w3c

import com.sce.runtime.ScriptLanguage
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.io.File

class GateEnginePairsTest {

    /**
     * One `engine:language` row of the gate's array.
     *
     * A data class rather than the raw string so a failure message can print
     * the pairs sorted and readable, and so a malformed row is a parse
     * failure here instead of an engine name with a colon in it.
     */
    private data class Pair(val engine: String, val language: String) : Comparable<Pair> {
        override fun toString(): String = "$engine:$language"
        override fun compareTo(other: Pair): Int = toString().compareTo(other.toString())
    }

    private companion object {
        /**
         * Relative to the repository root, which is this suite's `workingDir`
         * — the same anchor every other file-reading case here uses.
         */
        const val GATE_PATH = "scripts/gates/w3c-kotlin.sh"

        /** The array's name in the gate. Spelled once. */
        const val ARRAY_NAME = "KOTLIN_ENGINE_PAIRS"
    }

    /**
     * The pairs the gate declares, read out of the gate.
     *
     * A parse rather than an import, because the gate is Bash and there is no
     * shared format to put this in that would not itself be a third copy. The
     * parse is deliberately narrow: exactly one assignment line, in the array
     * form, or this fails. A gate that spells its array some other way is not
     * quietly accepted — the reader would then be describing a file it could
     * not actually read.
     */
    private fun declaredPairs(): Set<Pair> {
        val gate = File(GATE_PATH)
        assertTrue(
            gate.isFile,
            "$GATE_PATH is missing. This suite's workingDir is the repository " +
                "root, so a missing gate means the population claim below is " +
                "being checked against nothing."
        )

        val assignments = gate.readLines()
            .map { it.trim() }
            .filter { it.startsWith("$ARRAY_NAME=(") }
        assertEquals(
            1,
            assignments.size,
            "expected exactly one `$ARRAY_NAME=(...)` line in $GATE_PATH, found " +
                "${assignments.size}. Two would let the gate run one array while " +
                "this test reads the other; none means the gate stopped declaring " +
                "which pairs it covers."
        )

        val body = assignments.single()
            .substringAfter("(")
            .substringBeforeLast(")")
        val pairs = body.split(Regex("\\s+"))
            .filter { it.isNotBlank() }
            .map { row ->
                val parts = row.split(":")
                assertEquals(
                    2,
                    parts.size,
                    "row \"$row\" in $ARRAY_NAME is not `engine:language`. Each row " +
                        "names both halves because the engine alone does not say " +
                        "which artifact it was handed."
                )
                Pair(parts[0], parts[1])
            }
        return pairs.toSet().also {
            assertEquals(
                pairs.size,
                it.size,
                "$ARRAY_NAME repeats a row: $pairs. A duplicate spends the gate's " +
                    "minutes twice and covers nothing extra."
            )
        }
    }

    /**
     * Every route a machine can take into an engine is a row of the gate.
     *
     * The population: for each engine the suite can build, each language that
     * engine says it accepts. Both halves are answered by the code under test
     * rather than restated here — [W3CTestBase.KNOWN_ENGINES] is what
     * `createEngine` will build, and `acceptsLanguage` is what decides at run
     * time whether a `ScriptSource` is evaluated or refused.
     *
     * ⚠ Equality, not containment. An unsupported row is caught by the gate
     * running it, but a row for a pair that stopped being supported would
     * otherwise linger as minutes spent on a refusal; and equality is what
     * makes ADDING an engine, or teaching one an adapter, turn this red
     * instead of leaving the new route unmeasured.
     */
    @Test
    fun the_gate_runs_every_language_every_engine_accepts() {
        val supported = W3CTestBase.KNOWN_ENGINES.flatMap { name ->
            val engine = W3CTestBase.engineFor(name)
            ScriptLanguage.entries
                .filter { engine.acceptsLanguage(it) }
                .map { Pair(name, it.wireName) }
        }.toSet()

        assertTrue(
            supported.isNotEmpty(),
            "no engine accepts any language, which cannot be true while the " +
                "suite runs at all — the population derivation is broken, and an " +
                "empty population would make the comparison below pass for an " +
                "empty gate."
        )

        assertEquals(
            supported.sorted(),
            declaredPairs().sorted(),
            "$GATE_PATH's $ARRAY_NAME and the routes this backend actually has " +
                "disagree. A pair present here and absent there is an engine " +
                "route no lane measures; a pair present there and absent here is " +
                "minutes spent on a combination the engine refuses."
        )
    }

    /**
     * Every language the gate names is one the generator can emit for.
     *
     * The gate hands `--script-engine <language>` to `sce-codegen`, whose
     * vocabulary is [ScriptLanguage.wireName]. A row naming anything else
     * would fail at generation time with a message about a flag rather than
     * about the array, and only for the rows that need generating — the ones
     * matching the committed tree's language never reach the generator at all.
     */
    @Test
    fun every_declared_language_is_one_the_generator_spells() {
        val known = ScriptLanguage.entries.map { it.wireName }.toSet()
        for (pair in declaredPairs()) {
            assertTrue(
                known.contains(pair.language),
                "row \"$pair\" names language \"${pair.language}\", which is not a " +
                    "script-engine language ($known). The gate passes this string " +
                    "to `sce-codegen generate-w3c --script-engine`."
            )
        }
    }
}
